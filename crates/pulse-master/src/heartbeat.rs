use crate::alerts::AlertEngine;
use crate::storage::Storage;
use tokio::time::{Duration, interval};
use tracing::{info, warn};

pub async fn run_heartbeat_checker(storage: Storage, alerts: AlertEngine) {
    let mut tick = interval(Duration::from_secs(30));
    loop {
        tick.tick().await;
        if let Err(e) = check_once(&storage, &alerts).await {
            warn!(error = %e, "heartbeat check failed");
        }
    }
}

async fn check_once(storage: &Storage, alerts: &AlertEngine) -> anyhow::Result<()> {
    let offline = storage.get_offline_agents(45).await?;
    for agent_id in offline {
        info!(agent_id, "agent went OFFLINE");
        storage.set_agent_status(&agent_id, "offline").await?;
        alerts.fire_custom(&agent_id, "agent_offline", "Agent stopped sending metrics").await;
        alerts.evict_state(&agent_id).await;
    }
    Ok(())
}
