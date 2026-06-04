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
    let db_url = std::env::var("DATABASE_URL").ok();

    // Auto-detect data directory
    let data_dir =
        if std::path::Path::new("/var/lib/pulse").exists() || std::fs::create_dir_all("/var/lib/pulse").is_ok() {
            "/var/lib/pulse".to_string()
        } else {
            std::fs::create_dir_all("data").ok();
            "data".to_string()
        };

    // Auto-generate TLS certs if none exist
    let tls_dir = std::env::var("PULSE_TLS_DIR").unwrap_or_else(|_| format!("{data_dir}/certs"));
    let tls_enabled = if std::path::Path::new(&format!("{tls_dir}/ca.pem")).exists() {
        true
    } else if std::fs::create_dir_all(&tls_dir).is_ok() {
        info!("auto-generating TLS certificates...");
        match pulse_master::gen_dev_certs(&tls_dir) {
            Ok(()) => {
                info!(dir = %tls_dir, "TLS certificates generated");
                true
            }
            Err(e) => {
                warn!(error = %e, "failed to generate TLS certs, running insecure");
                false
            }
        }
    } else {
        false
    };

    // Auto-generate register token if not set (format: {sha256_of_ca_pem}::{random_secret})
    let register_token = if let Ok(t) = std::env::var("PULSE_REGISTER_TOKEN") {
        Some(t)
    } else {
        let token_path = format!("{data_dir}/register-token");
        if std::path::Path::new(&token_path).exists() {
            let t = std::fs::read_to_string(&token_path).unwrap_or_default().trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        } else {
            let secret: String = (0..32).map(|_| format!("{:02x}", rand_byte())).collect();
            let t = if tls_enabled {
                let ca_pem = std::fs::read_to_string(format!("{tls_dir}/ca.pem")).unwrap_or_default();
                let hash = ring::digest::digest(&ring::digest::SHA256, ca_pem.as_bytes());
                let ca_pin: String = hash.as_ref().iter().map(|b| format!("{:02x}", b)).collect();
                format!("{ca_pin}::{secret}")
            } else {
                secret
            };
            std::fs::write(&token_path, &t).ok();
            info!(token = %t, "register token generated (with CA pin)");
            Some(t)
        }
    };

    // Storage: Postgres if DATABASE_URL set, otherwise SQLite (zero-config default)
    let storage = {
        let url = db_url.as_deref();
        info!(backend = if url.is_some() { "postgres" } else { "sqlite" }, "connecting to database...");
        let s = Storage::connect(url).await?;
        s.migrate().await?;
        info!("database ready");
        s
    };

    let mut builder = if tls_enabled {
        let cert = std::fs::read_to_string(format!("{tls_dir}/server.pem"))?;
        let key = std::fs::read_to_string(format!("{tls_dir}/server-key.pem"))?;

        let tls = ServerTlsConfig::new().identity(Identity::from_pem(&cert, &key));

        info!(%addr, "pulse-master starting with TLS (auth via token)");
        Server::builder().tls_config(tls)?
    } else {
        info!(%addr, "pulse-master starting (insecure)");
        Server::builder()
    };

    let mut service = PulseService::new();
    service = service.with_storage(storage.clone());
    if let Some(ref token) = register_token {
        service = service.with_token(token.clone());
    }

    // Load or create CA for auto-provisioning
    if tls_enabled {
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
        let ca_pem = if tls_enabled { std::fs::read_to_string(format!("{tls_dir}/ca.pem")).ok() } else { None };
        let app = pulse_master::api::router_with_ca(storage.clone(), ca_pem);
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
    let retention_days: i64 = std::env::var("PULSE_RETENTION_DAYS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
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

fn rand_byte() -> u8 {
    use std::io::Read;
    let mut buf = [0u8; 1];
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)).unwrap_or_else(|_| {
        buf[0] = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos()
            & 0xFF) as u8;
    });
    buf[0]
}
