use pulse_proto::pb::{
    CommandRequest, CommandResponse, MetricsAck, MetricsPayload, RegisterRequest, RegisterResponse,
    pulse_agent_server::{PulseAgent, PulseAgentServer},
};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, Streaming};
use tracing::info;

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct PulseService {
    next_id: AtomicU64,
    cmd_tx: Arc<tokio::sync::Mutex<Option<mpsc::Sender<CommandRequest>>>>,
}

impl Default for PulseService {
    fn default() -> Self {
        Self::new()
    }
}

impl PulseService {
    pub fn new() -> Self {
        Self { next_id: AtomicU64::new(1), cmd_tx: Arc::new(tokio::sync::Mutex::new(None)) }
    }

    pub fn into_server(self) -> PulseAgentServer<Self> {
        PulseAgentServer::new(self)
    }

    pub fn command_sender(&self) -> Arc<tokio::sync::Mutex<Option<mpsc::Sender<CommandRequest>>>> {
        self.cmd_tx.clone()
    }
}

#[tonic::async_trait]
impl PulseAgent for PulseService {
    async fn register(&self, req: Request<RegisterRequest>) -> Result<Response<RegisterResponse>, Status> {
        let r = req.into_inner();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let agent_id = format!("agent-{:04x}", id);
        info!(agent_id, hostname = r.hostname, os = r.os, arch = r.arch, version = r.agent_version, "agent registered");
        Ok(Response::new(RegisterResponse { agent_id, metrics_interval_ms: 10_000, ca_cert: Vec::new() }))
    }

    async fn stream_metrics(&self, req: Request<Streaming<MetricsPayload>>) -> Result<Response<MetricsAck>, Status> {
        let mut stream = req.into_inner();
        let mut count = 0u64;

        while let Some(payload) = stream.message().await? {
            count += 1;
            if let Some(ref cpu) = payload.cpu {
                info!(
                    agent = payload.agent_id,
                    cpu_pct = format!("{:.1}%", cpu.usage_total_bp as f32 / 100.0),
                    count,
                    "metrics"
                );
            }
            if let Some(ref mem) = payload.memory {
                let used_mb = mem.used_bytes / 1_048_576;
                let total_mb = mem.total_bytes / 1_048_576;
                info!(agent = payload.agent_id, used_mb, total_mb, "memory");
            }
        }

        info!(count, "metrics stream ended");
        Ok(Response::new(MetricsAck { ok: true, next_interval_ms: 10_000 }))
    }

    type CommandChannelStream = Pin<Box<dyn Stream<Item = Result<CommandRequest, Status>> + Send>>;

    async fn command_channel(
        &self,
        req: Request<Streaming<CommandResponse>>,
    ) -> Result<Response<Self::CommandChannelStream>, Status> {
        let (tx, rx) = mpsc::channel(16);

        // Store sender for external command injection
        *self.cmd_tx.lock().await = Some(tx);

        // Process responses from agent
        let mut response_stream = req.into_inner();
        tokio::spawn(async move {
            while let Ok(Some(resp)) = response_stream.message().await {
                if let Some(payload) = resp.payload {
                    match payload {
                        pulse_proto::pb::command_response::Payload::Output(chunk) => {
                            let text = String::from_utf8_lossy(&chunk.data);
                            info!(request_id = resp.request_id, stream = chunk.stream, %text, "output");
                        }
                        pulse_proto::pb::command_response::Payload::Result(result) => {
                            info!(
                                request_id = resp.request_id,
                                exit_code = result.exit_code,
                                timed_out = result.timed_out,
                                "command completed"
                            );
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(rx).map(Ok);
        Ok(Response::new(Box::pin(stream)))
    }
}
