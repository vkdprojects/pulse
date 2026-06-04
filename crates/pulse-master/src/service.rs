use crate::alerts::AlertEngine;
use crate::ca::CaBundle;
use crate::storage::Storage;
use crate::updates::UpdateState;
use pulse_proto::pb::{
    CommandRequest, CommandResponse, MetricsAck, MetricsPayload, RegisterRequest, RegisterResponse, UpdateSignal,
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
    storage: Option<Storage>,
    ca: Option<Arc<CaBundle>>,
    alerts: AlertEngine,
    register_token: Option<String>,
    update_state: Option<UpdateState>,
}

impl Default for PulseService {
    fn default() -> Self {
        Self::new()
    }
}

impl PulseService {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            cmd_tx: Arc::new(tokio::sync::Mutex::new(None)),
            storage: None,
            ca: None,
            alerts: AlertEngine::new(),
            register_token: std::env::var("PULSE_REGISTER_TOKEN").ok(),
            update_state: None,
        }
    }

    pub fn with_storage(mut self, storage: Storage) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn with_ca(mut self, ca: CaBundle) -> Self {
        self.ca = Some(Arc::new(ca));
        self
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.register_token = Some(token);
        self
    }

    pub fn with_update_state(mut self, state: UpdateState) -> Self {
        self.update_state = Some(state);
        self
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

        // Auth check
        if let Some(ref expected) = self.register_token
            && r.auth_token != *expected
        {
            return Err(Status::unauthenticated("invalid register token"));
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let agent_id = format!("agent-{:04x}", id);
        info!(agent_id, hostname = r.hostname, os = r.os, arch = r.arch, version = r.agent_version, "agent registered");

        if let Some(ref db) = self.storage {
            db.upsert_agent(&agent_id, &r).await.map_err(|e| Status::internal(e.to_string()))?;
        }

        let (ca_cert, client_cert, client_key) = if let Some(ref ca) = self.ca {
            let issued = ca.issue_client_cert(&agent_id).map_err(|e| Status::internal(e.to_string()))?;
            (ca.ca_cert_pem.as_bytes().to_vec(), issued.cert_pem.into_bytes(), issued.key_pem.into_bytes())
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        Ok(Response::new(RegisterResponse { agent_id, metrics_interval_ms: 10_000, ca_cert, client_cert, client_key }))
    }

    async fn stream_metrics(&self, req: Request<Streaming<MetricsPayload>>) -> Result<Response<MetricsAck>, Status> {
        let mut stream = req.into_inner();
        let mut count = 0u64;
        let mut last_agent_id = String::new();

        while let Some(payload) = stream.message().await? {
            count += 1;
            last_agent_id.clone_from(&payload.agent_id);
            self.alerts.check(&payload).await;
            if let Some(ref db) = self.storage
                && let Err(e) = db.insert_metrics(&payload).await
            {
                tracing::warn!(error = %e, "failed to persist metrics");
            }
            if let Some(ref cpu) = payload.cpu
                && count % 10 == 1
            {
                info!(
                    agent = payload.agent_id,
                    cpu_pct = format!("{:.1}%", cpu.usage_total_bp as f32 / 100.0),
                    count,
                    "metrics"
                );
            }
        }

        // Build update signal if applicable — look up agent version from DB
        let update = if let Some(ref us) = self.update_state {
            let config = us.config.read().await;
            let agent_version = if let Some(ref db) = self.storage {
                db.get_agent_version(&last_agent_id).await.unwrap_or_default()
            } else {
                String::new()
            };
            if us.should_signal(&last_agent_id, &agent_version, &config) {
                config.target_version.as_ref().map(|v| {
                    let arch = "x86_64"; // fallback; agent will use its own arch
                    let path = us.binary_path(v, arch);
                    let sha = path
                        .exists()
                        .then(|| {
                            std::fs::read(&path).ok().map(|data| {
                                let hash = ring::digest::digest(&ring::digest::SHA256, &data);
                                hash.as_ref().iter().map(|b| format!("{:02x}", b)).collect::<String>()
                            })
                        })
                        .flatten()
                        .unwrap_or_default();
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    UpdateSignal { version: v.clone(), sha256: sha, size_bytes: size, urgent: config.urgent }
                })
            } else {
                None
            }
        } else {
            None
        };

        info!(count, "metrics stream ended");
        Ok(Response::new(MetricsAck { ok: true, next_interval_ms: 10_000, update }))
    }

    type CommandChannelStream = Pin<Box<dyn Stream<Item = Result<CommandRequest, Status>> + Send>>;

    async fn command_channel(
        &self,
        req: Request<Streaming<CommandResponse>>,
    ) -> Result<Response<Self::CommandChannelStream>, Status> {
        let (tx, rx) = mpsc::channel(16);
        *self.cmd_tx.lock().await = Some(tx);

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
