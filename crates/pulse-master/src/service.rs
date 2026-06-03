use pulse_proto::pb::{
    CommandRequest, CommandResponse, MetricsAck, MetricsPayload, RegisterRequest, RegisterResponse,
    pulse_agent_server::{PulseAgent, PulseAgentServer},
};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, Streaming};

use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct PulseService {
    next_id: AtomicU64,
}

impl Default for PulseService {
    fn default() -> Self {
        Self::new()
    }
}

impl PulseService {
    pub fn new() -> Self {
        Self { next_id: AtomicU64::new(1) }
    }

    pub fn into_server(self) -> PulseAgentServer<Self> {
        PulseAgentServer::new(self)
    }
}

#[tonic::async_trait]
impl PulseAgent for PulseService {
    async fn register(&self, req: Request<RegisterRequest>) -> Result<Response<RegisterResponse>, Status> {
        let _r = req.into_inner();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        Ok(Response::new(RegisterResponse {
            agent_id: format!("agent-{:04x}", id),
            metrics_interval_ms: 10_000,
            ca_cert: Vec::new(),
        }))
    }

    async fn stream_metrics(&self, req: Request<Streaming<MetricsPayload>>) -> Result<Response<MetricsAck>, Status> {
        let mut stream = req.into_inner();
        while let Some(_payload) = stream.message().await? {}
        Ok(Response::new(MetricsAck { ok: true, next_interval_ms: 10_000 }))
    }

    type CommandChannelStream = Pin<Box<dyn Stream<Item = Result<CommandRequest, Status>> + Send>>;

    async fn command_channel(
        &self,
        _req: Request<Streaming<CommandResponse>>,
    ) -> Result<Response<Self::CommandChannelStream>, Status> {
        let (_tx, rx) = mpsc::channel(16);
        let stream = ReceiverStream::new(rx).map(Ok);
        Ok(Response::new(Box::pin(stream)))
    }
}
