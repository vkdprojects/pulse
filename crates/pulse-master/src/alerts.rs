use crate::storage::Storage;
use pulse_proto::pb::MetricsPayload;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;

#[derive(Clone)]
pub struct AlertEngine {
    state: Arc<Mutex<HashMap<String, AlertState>>>,
    webhook_url: Option<String>,
    cpu_threshold_bp: u32,
    mem_threshold_pct: u32,
    bw_threshold_bytes: u64,
    storage: Option<Storage>,
}

struct AlertState {
    cpu_high_count: u32,
    mem_high_count: u32,
    bandwidth_high_count: u32,
}

impl Default for AlertEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertEngine {
    pub fn new() -> Self {
        let webhook_url = std::env::var("PULSE_ALERT_WEBHOOK").ok();
        let cpu_threshold = std::env::var("PULSE_ALERT_CPU_PCT").ok().and_then(|s| s.parse().ok()).unwrap_or(90);
        let mem_threshold = std::env::var("PULSE_ALERT_MEM_PCT").ok().and_then(|s| s.parse().ok()).unwrap_or(90);
        let bw_mbps: u64 = std::env::var("PULSE_ALERT_BW_MBPS").ok().and_then(|s| s.parse().ok()).unwrap_or(100);

        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            webhook_url,
            cpu_threshold_bp: cpu_threshold * 100,
            mem_threshold_pct: mem_threshold,
            bw_threshold_bytes: bw_mbps * 1_000_000,
            storage: None,
        }
    }

    pub fn with_storage(mut self, storage: Storage) -> Self {
        self.storage = Some(storage);
        self
    }

    pub async fn check(&self, payload: &MetricsPayload) {
        let mut alerts_to_fire: Vec<(String, String)> = Vec::new();

        {
            let mut state = self.state.lock().await;
            let entry = state.entry(payload.agent_id.clone()).or_insert(AlertState {
                cpu_high_count: 0,
                mem_high_count: 0,
                bandwidth_high_count: 0,
            });

            if let Some(ref cpu) = payload.cpu {
                if cpu.usage_total_bp > self.cpu_threshold_bp {
                    entry.cpu_high_count += 1;
                    if entry.cpu_high_count == 3 {
                        alerts_to_fire.push((
                            "cpu_high".into(),
                            format!(
                                "CPU at {:.1}% (threshold: {}%)",
                                cpu.usage_total_bp as f32 / 100.0,
                                self.cpu_threshold_bp / 100
                            ),
                        ));
                    }
                } else {
                    entry.cpu_high_count = 0;
                }
            }

            if let Some(ref mem) = payload.memory
                && mem.total_bytes > 0
            {
                let used_pct = (mem.used_bytes * 100 / mem.total_bytes) as u32;
                if used_pct > self.mem_threshold_pct {
                    entry.mem_high_count += 1;
                    if entry.mem_high_count == 3 {
                        alerts_to_fire.push((
                            "mem_high".into(),
                            format!("Memory at {}% (threshold: {}%)", used_pct, self.mem_threshold_pct),
                        ));
                    }
                } else {
                    entry.mem_high_count = 0;
                }
            }

            let max_bw = payload.net.iter().map(|n| n.rx_bytes_sec.max(n.tx_bytes_sec)).max().unwrap_or(0);
            if max_bw > self.bw_threshold_bytes {
                entry.bandwidth_high_count += 1;
                if entry.bandwidth_high_count == 3 {
                    alerts_to_fire.push((
                        "bandwidth_high".into(),
                        format!(
                            "Bandwidth at {} MB/s (threshold: {} MB/s)",
                            max_bw / 1_000_000,
                            self.bw_threshold_bytes / 1_000_000
                        ),
                    ));
                }
            } else {
                entry.bandwidth_high_count = 0;
            }
        } // lock dropped

        for (alert_type, msg) in &alerts_to_fire {
            self.fire(&payload.agent_id, alert_type, msg).await;
        }
    }

    async fn fire(&self, agent_id: &str, alert_type: &str, message: &str) {
        warn!(agent_id, alert_type, message, "ALERT FIRED");

        if let Some(ref storage) = self.storage
            && let Err(e) = storage.insert_alert(agent_id, alert_type, message).await
        {
            warn!(error = %e, "failed to persist alert");
        }

        if let Some(ref url) = self.webhook_url {
            let body = format_webhook_body(url, agent_id, alert_type, message);
            let url = url.clone();
            tokio::spawn(async move {
                let _ = post_webhook(&url, &body).await;
            });
        }
    }

    pub async fn fire_custom(&self, agent_id: &str, alert_type: &str, message: &str) {
        self.fire(agent_id, alert_type, message).await;
    }

    pub async fn evict_state(&self, agent_id: &str) {
        self.state.lock().await.remove(agent_id);
    }
}

fn format_webhook_body(url: &str, agent_id: &str, alert_type: &str, message: &str) -> String {
    if url.contains("api.telegram.org") {
        // Telegram Bot API format
        format!(
            r#"{{"chat_id":"{}","text":"🚨 *PULSE ALERT*\n\nAgent: `{}`\nType: {}\n{}","parse_mode":"Markdown"}}"#,
            std::env::var("PULSE_ALERT_TELEGRAM_CHAT_ID").unwrap_or_default(),
            agent_id,
            alert_type,
            message
        )
    } else if url.contains("hooks.slack.com") || url.contains("discord.com/api/webhooks") {
        // Slack/Discord webhook format (compatible)
        format!(r#"{{"content":"🚨 **PULSE ALERT**\n\nAgent: `{}`\nType: {}\n{}"}}"#, agent_id, alert_type, message)
    } else {
        // Generic webhook
        format!(r#"{{"agent":"{}","type":"{}","message":"{}","timestamp":"{}"}}"#, agent_id, alert_type, message, now())
    }
}

fn now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

async fn post_webhook(url: &str, body: &str) -> anyhow::Result<()> {
    let mut stream = tokio::net::TcpStream::connect(parse_host_port(url)?).await?;
    let path = url.find('/').map(|i| &url[i..]).unwrap_or("/");
    let host = parse_host_port(url)?.0;
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path,
        host,
        body.len(),
        body
    );
    use tokio::io::AsyncWriteExt;
    stream.write_all(req.as_bytes()).await?;
    Ok(())
}

fn parse_host_port(url: &str) -> anyhow::Result<(&str, u16)> {
    let url = url.strip_prefix("http://").unwrap_or(url);
    let url = url.strip_prefix("https://").unwrap_or(url);
    let (host_port, _) = url.split_once('/').unwrap_or((url, ""));
    let (host, port) = host_port.split_once(':').unwrap_or((host_port, "80"));
    Ok((host, port.parse().unwrap_or(80)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulse_proto::pb::{CpuMetrics, MemoryMetrics, MetricsPayload};

    fn make_payload(cpu_bp: u32, mem_used: u64, mem_total: u64) -> MetricsPayload {
        MetricsPayload {
            timestamp_ms: 1000,
            agent_id: "test-agent".into(),
            cpu: Some(CpuMetrics { usage_total_bp: cpu_bp, ..Default::default() }),
            memory: Some(MemoryMetrics { total_bytes: mem_total, used_bytes: mem_used, ..Default::default() }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_alert_does_not_fire_under_threshold() {
        let engine = AlertEngine::new();
        // 50% CPU — under default 90%
        engine.check(&make_payload(5000, 4_000_000_000, 8_000_000_000)).await;
        engine.check(&make_payload(5000, 4_000_000_000, 8_000_000_000)).await;
        engine.check(&make_payload(5000, 4_000_000_000, 8_000_000_000)).await;
        // Counter should stay at 0 because under threshold
        let state = engine.state.lock().await;
        assert_eq!(state.get("test-agent").unwrap().cpu_high_count, 0);
        assert_eq!(state.get("test-agent").unwrap().mem_high_count, 0);
    }

    #[tokio::test]
    async fn test_alert_state_resets_on_recovery() {
        let engine = AlertEngine::new();
        // 2 high readings
        engine.check(&make_payload(9500, 0, 1)).await;
        engine.check(&make_payload(9500, 0, 1)).await;
        // Recovery
        engine.check(&make_payload(5000, 0, 1)).await;
        // High again — counter should reset, not fire on 1st
        engine.check(&make_payload(9500, 0, 1)).await;
        let state = engine.state.lock().await;
        assert_eq!(state.get("test-agent").unwrap().cpu_high_count, 1);
    }

    #[test]
    fn test_webhook_format_telegram() {
        let body =
            format_webhook_body("https://api.telegram.org/bot123/sendMessage", "agent-01", "cpu_high", "CPU at 95%");
        assert!(body.contains("chat_id"));
        assert!(body.contains("agent-01"));
    }

    #[test]
    fn test_webhook_format_slack() {
        let body = format_webhook_body("https://hooks.slack.com/services/XXX", "agent-01", "mem_high", "Memory at 92%");
        assert!(body.contains("content"));
        assert!(body.contains("agent-01"));
    }

    #[test]
    fn test_webhook_format_generic() {
        let body = format_webhook_body("https://my-service.com/webhook", "agent-01", "cpu_high", "test");
        assert!(body.contains("timestamp"));
        assert!(body.contains("agent-01"));
    }
}
