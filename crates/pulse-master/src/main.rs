use anyhow::Result;
use pulse_master::service::PulseService;
use pulse_master::storage::Storage;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())).init();

    if std::env::args().nth(1).as_deref() == Some("gen-certs") {
        let dir = std::env::args().nth(2).unwrap_or_else(|| ".certs".into());
        pulse_master::gen_dev_certs(&dir)?;
        println!("certificates generated in {dir}/");
        return Ok(());
    }

    let addr = std::env::var("PULSE_LISTEN").unwrap_or_else(|_| "[::1]:50051".into()).parse()?;
    let tls_dir = std::env::var("PULSE_TLS_DIR").ok();
    let db_url = std::env::var("DATABASE_URL").ok();

    // Connect to TimescaleDB if configured
    let storage = if let Some(ref url) = db_url {
        info!("connecting to TimescaleDB...");
        let s = Storage::connect(url).await?;
        s.migrate().await?;
        info!("database ready");
        Some(s)
    } else {
        info!("no DATABASE_URL — running without persistence");
        None
    };

    let mut builder = if let Some(ref dir) = tls_dir {
        let cert = std::fs::read_to_string(format!("{dir}/server.pem"))?;
        let key = std::fs::read_to_string(format!("{dir}/server-key.pem"))?;
        let ca_cert = std::fs::read_to_string(format!("{dir}/ca.pem"))?;

        let tls = ServerTlsConfig::new()
            .identity(Identity::from_pem(&cert, &key))
            .client_ca_root(tonic::transport::Certificate::from_pem(&ca_cert));

        info!(%addr, "pulse-master starting with mTLS");
        Server::builder().tls_config(tls)?
    } else {
        info!(%addr, "pulse-master starting (insecure)");
        Server::builder()
    };

    let mut service = PulseService::new();
    if let Some(ref s) = storage {
        service = service.with_storage(s.clone());
    }

    // Load or create CA for auto-provisioning
    if tls_dir.is_some() {
        let ca = pulse_master::ca::CaBundle::new().map_err(|e| anyhow::anyhow!("CA init failed: {e}"))?;
        info!("CA initialized for agent auto-provisioning");
        service = service.with_ca(ca);
    }

    // Background: agent offline detection
    if let Some(s) = storage {
        let alerts = pulse_master::alerts::AlertEngine::new();
        tokio::spawn(pulse_master::heartbeat::run_heartbeat_checker(s, alerts));
    }

    // Health endpoint (HTTP)
    tokio::spawn(async {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
        info!("health endpoint on :8080/health");
        loop {
            if let Ok((mut stream, _)) = listener.accept().await {
                use tokio::io::AsyncWriteExt;
                let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\"}\n";
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        }
    });

    builder.add_service(service.into_server()).serve(addr).await?;
    Ok(())
}
