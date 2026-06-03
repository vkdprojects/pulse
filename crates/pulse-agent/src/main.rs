use anyhow::Result;
use pulse_proto::pb::{RegisterRequest, pulse_agent_client::PulseAgentClient};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let endpoint = std::env::var("PULSE_MASTER_URL").unwrap_or_else(|_| "http://[::1]:50051".into());

    println!("pulse-agent connecting to {endpoint}");
    let mut client = PulseAgentClient::connect(endpoint).await?;

    let resp = client
        .register(RegisterRequest {
            hostname: gethostname(),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            agent_version: env!("CARGO_PKG_VERSION").into(),
            labels: vec![],
        })
        .await?;

    let r = resp.into_inner();
    println!("registered as {} (interval={}ms)", r.agent_id, r.metrics_interval_ms);

    tokio::signal::ctrl_c().await?;
    Ok(())
}

fn gethostname() -> String {
    hostname::get().map(|h| h.to_string_lossy().into_owned()).unwrap_or_else(|_| "unknown".into())
}
