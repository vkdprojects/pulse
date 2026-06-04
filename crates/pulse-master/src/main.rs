use anyhow::Result;
use pulse_master::service::PulseService;
use pulse_master::storage::Storage;
use tokio::time::{Duration, interval};
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tracing::{info, warn};

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

    // Storage: Postgres if DATABASE_URL set, otherwise SQLite (zero-config default)
    let storage = {
        let url = db_url.as_deref();
        info!(backend = if url.is_some() { "postgres" } else { "sqlite" }, "connecting to database...");
        let s = Storage::connect(url).await?;
        s.migrate().await?;
        info!("database ready");
        s
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
    service = service.with_storage(storage.clone());

    // Load or create CA for auto-provisioning
    if tls_dir.is_some() {
        let ca = pulse_master::ca::CaBundle::new().map_err(|e| anyhow::anyhow!("CA init failed: {e}"))?;
        info!("CA initialized for agent auto-provisioning");
        service = service.with_ca(ca);
    }

    // Background: agent offline detection + retention
    {
        let alerts = pulse_master::alerts::AlertEngine::new();
        tokio::spawn(pulse_master::heartbeat::run_heartbeat_checker(storage.clone(), alerts));
        tokio::spawn(run_retention_worker(storage.clone()));
    }

    // REST API (HTTP on :8080)
    {
        let app = pulse_master::api::router(storage.clone());
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
            info!("REST API on :8080");
            axum::serve(listener, app).await.unwrap();
        });
    }

    builder.add_service(service.into_server()).serve(addr).await?;
    Ok(())
}

async fn run_retention_worker(storage: Storage) {
    let retention_days: i64 =
        std::env::var("PULSE_RETENTION_DAYS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    let mut tick = interval(Duration::from_secs(3600));
    loop {
        tick.tick().await;
        if let Err(e) = storage.delete_old_data(retention_days).await {
            warn!(error = %e, "retention cleanup failed");
        } else {
            info!(retention_days, "retention cleanup complete");
        }
    }
}
