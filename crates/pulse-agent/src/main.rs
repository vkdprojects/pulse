mod config;
mod telemetry;
mod worker;

use anyhow::Result;
use config::Config;
use pulse_proto::pb::{CommandResponse, MetricsPayload, RegisterRequest, pulse_agent_client::PulseAgentClient};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use tracing::{error, info, warn};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())).init();

    let config_path = std::env::args().nth(1);
    let config = Config::load(config_path.as_deref())?;
    info!(endpoint = %config.master_endpoint, interval = config.metrics_interval_ms, "pulse-agent starting");

    run_agent(config).await
}

async fn run_agent(config: Config) -> Result<()> {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(60);

    loop {
        match connect_and_stream(&config).await {
            Ok(()) => {
                info!("connection closed gracefully");
                return Ok(());
            }
            Err(e) => {
                error!(error = %e, "connection failed, retrying in {:?}", backoff);
                sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
            }
        }
    }
}

async fn connect_and_stream(config: &Config) -> Result<()> {
    let channel = build_channel(config).await?;
    let mut client = PulseAgentClient::new(channel);

    let resp = client
        .register(RegisterRequest {
            hostname: hostname::get().map(|h| h.to_string_lossy().into_owned()).unwrap_or_else(|_| "unknown".into()),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            agent_version: env!("CARGO_PKG_VERSION").into(),
            labels: config.labels.clone(),
        })
        .await?;

    let reg = resp.into_inner();
    let agent_id = reg.agent_id;
    let interval = Duration::from_millis(reg.metrics_interval_ms.max(1000).into());
    info!(agent_id, ?interval, "registered with master");

    // Metrics stream
    let (metrics_tx, metrics_rx) = mpsc::channel::<MetricsPayload>(32);
    let metrics_stream = ReceiverStream::new(metrics_rx);

    let mut client_clone = client.clone();
    let metrics_handle = tokio::spawn(async move {
        match client_clone.stream_metrics(metrics_stream).await {
            Ok(resp) => {
                let ack = resp.into_inner();
                info!(ok = ack.ok, "metrics stream ended");
            }
            Err(e) => warn!(error = %e, "metrics stream error"),
        }
    });

    // Command channel
    let (cmd_tx, cmd_rx) = mpsc::channel::<CommandResponse>(32);
    let cmd_stream = ReceiverStream::new(cmd_rx);

    let cmd_tx_clone = cmd_tx.clone();
    let cmd_handle = tokio::spawn(async move {
        match client.command_channel(cmd_stream).await {
            Ok(resp) => {
                let mut stream = resp.into_inner();
                while let Ok(Some(cmd_req)) = stream.message().await {
                    info!(request_id = %cmd_req.request_id, "received command");
                    let tx = cmd_tx_clone.clone();
                    tokio::spawn(async move {
                        if let Err(e) = worker::execute(cmd_req, tx).await {
                            error!(error = %e, "worker execution failed");
                        }
                    });
                }
            }
            Err(e) => warn!(error = %e, "command channel error"),
        }
    });

    // Collect & send metrics loop
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    let mut first = true;
    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("shutdown signal received");
                drop(metrics_tx);
                drop(cmd_tx);
                break;
            }
            _ = sleep(if first { Duration::from_millis(100) } else { interval }) => {
                first = false;
                match collect_snapshot(&agent_id).await {
                    Ok(payload) => {
                        if metrics_tx.send(payload).await.is_err() {
                            warn!("metrics channel closed");
                            break;
                        }
                    }
                    Err(e) => warn!(error = %e, "telemetry collection failed"),
                }
            }
        }
    }

    let _ = metrics_handle.await;
    let _ = cmd_handle.await;
    Ok(())
}

async fn collect_snapshot(agent_id: &str) -> Result<MetricsPayload> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as u64;

    let (cpu, memory, net, disks, system) = tokio::join!(
        telemetry::collect_cpu(),
        telemetry::collect_memory(),
        telemetry::collect_net(),
        telemetry::collect_disks(),
        telemetry::collect_system(),
    );

    Ok(MetricsPayload {
        timestamp_ms: now,
        agent_id: agent_id.into(),
        cpu: cpu.ok(),
        memory: memory.ok(),
        net: net.unwrap_or_default(),
        disks: disks.unwrap_or_default(),
        system: system.ok(),
    })
}

async fn build_channel(config: &Config) -> Result<Channel> {
    let endpoint = Channel::from_shared(config.master_endpoint.clone())?;

    if config.tls_enabled() {
        let tls = &config.tls;
        let ca = tokio::fs::read_to_string(tls.ca_cert.as_ref().unwrap()).await?;
        let cert = tokio::fs::read_to_string(tls.client_cert.as_ref().unwrap()).await?;
        let key = tokio::fs::read_to_string(tls.client_key.as_ref().unwrap()).await?;

        let tls_config =
            ClientTlsConfig::new().ca_certificate(Certificate::from_pem(&ca)).identity(Identity::from_pem(&cert, &key));

        Ok(endpoint.tls_config(tls_config)?.connect().await?)
    } else {
        Ok(endpoint.connect().await?)
    }
}
