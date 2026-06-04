mod config;
mod containers;
mod telemetry;
mod worker;

use anyhow::Result;
use config::Config;
use pulse_proto::pb::{CommandResponse, MetricsPayload, RegisterRequest, pulse_agent_client::PulseAgentClient};
use std::collections::VecDeque;
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
    let mut backoff_secs = 1u64;
    let mut buffer: VecDeque<MetricsPayload> = VecDeque::with_capacity(100);

    loop {
        let result = connect_and_stream(&config, &mut buffer).await;
        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                let backoff = Duration::from_secs(backoff_secs);
                error!(error = %e, buffer_size = buffer.len(), ?backoff, "connection lost, reconnecting");
                sleep(backoff).await;
                backoff_secs = (backoff_secs * 2).min(60);
            }
        }
    }
}

async fn connect_and_stream(config: &Config, buffer: &mut VecDeque<MetricsPayload>) -> Result<()> {
    let channel = build_channel(config).await?;
    let mut client = PulseAgentClient::new(channel);

    let resp = client
        .register(RegisterRequest {
            hostname: hostname::get().map(|h| h.to_string_lossy().into_owned()).unwrap_or_else(|_| "unknown".into()),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            agent_version: env!("CARGO_PKG_VERSION").into(),
            labels: config.labels.clone(),
            auth_token: config.auth_token.clone(),
        })
        .await?;

    let reg = resp.into_inner();
    let agent_id = reg.agent_id;
    let interval = Duration::from_millis(reg.metrics_interval_ms.max(1000).into());
    info!(agent_id, ?interval, "registered with master");

    // Persist client certs if provisioned by master
    if !reg.client_cert.is_empty() && !reg.client_key.is_empty() && !reg.ca_cert.is_empty() {
        if let Err(e) = save_provisioned_certs(config, &reg.ca_cert, &reg.client_cert, &reg.client_key) {
            warn!(error = %e, "failed to save client certificates");
        } else {
            info!("client certificate saved, will use mTLS on next connection");
        }
    }

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

    // Drain buffered metrics from previous disconnection
    if !buffer.is_empty() {
        info!(count = buffer.len(), "draining buffered metrics");
        while let Some(payload) = buffer.pop_front() {
            if metrics_tx.send(payload).await.is_err() {
                break;
            }
        }
    }

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
                        if metrics_tx.send(payload.clone()).await.is_err() {
                            // Buffer for next reconnection
                            if buffer.len() >= 100 {
                                buffer.pop_front();
                            }
                            buffer.push_back(payload);
                            warn!(buffer_size = buffer.len(), "metrics channel closed, buffering");
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

    let (cpu, memory, net, disks, system, ctrs, disk_io, procs, temps, gpus, dhealth) = tokio::join!(
        telemetry::collect_cpu(),
        telemetry::collect_memory(),
        telemetry::collect_net(),
        telemetry::collect_disks(),
        telemetry::collect_system(),
        containers::collect_containers(),
        telemetry::collect_disk_io(),
        telemetry::collect_top_processes(),
        telemetry::collect_temperatures(),
        telemetry::collect_gpus(),
        telemetry::collect_disk_health(),
    );

    Ok(MetricsPayload {
        timestamp_ms: now,
        agent_id: agent_id.into(),
        cpu: cpu.ok(),
        memory: memory.ok(),
        net: net.unwrap_or_default(),
        disks: disks.unwrap_or_default(),
        system: system.ok(),
        containers: ctrs.unwrap_or_default(),
        disk_io: disk_io.unwrap_or_default(),
        top_processes: procs.unwrap_or_default(),
        temperatures: temps.unwrap_or_default(),
        gpus: gpus.unwrap_or_default(),
        disk_health: dhealth.unwrap_or_default(),
        battery_pct: telemetry::collect_battery().await,
        ssh_sessions: telemetry::collect_ssh_sessions().await,
    })
}

async fn build_channel(config: &Config) -> Result<Channel> {
    let endpoint = Channel::from_shared(config.master_endpoint.clone())?;

    if config.tls_enabled() {
        let tls = &config.tls;
        let ca = tokio::fs::read_to_string(tls.ca_cert.as_ref().unwrap()).await?;
        let cert = tokio::fs::read_to_string(tls.client_cert.as_ref().unwrap()).await?;
        let key = tokio::fs::read_to_string(tls.client_key.as_ref().unwrap()).await?;

        let tls_config = ClientTlsConfig::new()
            .domain_name(tls.domain_name.as_deref().unwrap_or("localhost"))
            .ca_certificate(Certificate::from_pem(&ca))
            .identity(Identity::from_pem(&cert, &key));

        Ok(endpoint.tls_config(tls_config)?.connect().await?)
    } else if config.master_endpoint.starts_with("https://") {
        // Bootstrap: fetch CA cert from master's HTTP API, validate CA pin, then connect
        let ca_pem = fetch_and_verify_ca(&config.master_endpoint, &config.auth_token).await?;
        let tls_config = ClientTlsConfig::new().domain_name("localhost").ca_certificate(Certificate::from_pem(&ca_pem));
        Ok(endpoint.tls_config(tls_config)?.connect().await?)
    } else {
        Ok(endpoint.connect().await?)
    }
}

/// Fetch CA cert from master's REST API and validate against token's CA pin
async fn fetch_and_verify_ca(grpc_endpoint: &str, auth_token: &str) -> Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let host_port = grpc_endpoint.trim_start_matches("https://");
    let host = host_port.split(':').next().unwrap_or("localhost");
    let addr = format!("{host}:8080");

    info!(addr = %addr, "fetching CA certificate from master...");

    let mut stream = tokio::net::TcpStream::connect(&addr).await?;
    let req = format!("GET /api/ca HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    let response = String::from_utf8_lossy(&buf);

    let body = response.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default();

    if !body.contains("BEGIN CERTIFICATE") {
        anyhow::bail!("invalid CA response from master at {addr}: {}", &body[..body.len().min(200)]);
    }

    // Validate CA pin if token contains one (format: {ca_pin}::{secret})
    if let Some((ca_pin, _secret)) = auth_token.split_once("::") {
        let hash = ring::digest::digest(&ring::digest::SHA256, body.as_bytes());
        let computed: String = hash.as_ref().iter().map(|b| format!("{:02x}", b)).collect();
        if computed != ca_pin {
            anyhow::bail!("CA pin mismatch — possible MITM (expected {ca_pin}, got {computed})");
        }
        info!("CA certificate pin verified");
    }

    Ok(body)
}

/// Save provisioned certs to disk and update config TOML with [tls] section
fn save_provisioned_certs(_config: &Config, ca_cert: &[u8], client_cert: &[u8], client_key: &[u8]) -> Result<()> {
    let cert_dir = std::path::PathBuf::from("/etc/pulse/certs");
    std::fs::create_dir_all(&cert_dir)?;

    let ca_path = cert_dir.join("ca.pem");
    let cert_path = cert_dir.join("client.pem");
    let key_path = cert_dir.join("client-key.pem");

    std::fs::write(&ca_path, ca_cert)?;
    std::fs::write(&cert_path, client_cert)?;
    std::fs::write(&key_path, client_key)?;

    // Update config TOML to include [tls] section
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "/etc/pulse-agent.toml".into());
    if std::path::Path::new(&config_path).exists() {
        let content = std::fs::read_to_string(&config_path)?;
        if !content.contains("[tls]") {
            let tls_section = format!(
                "\n[tls]\nca_cert = \"{}\"\nclient_cert = \"{}\"\nclient_key = \"{}\"\n",
                ca_path.display(),
                cert_path.display(),
                key_path.display()
            );
            std::fs::write(&config_path, format!("{content}{tls_section}"))?;
        }
    }

    Ok(())
}
