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
}

struct AlertState {
    cpu_high_count: u32,
    mem_high_count: u32,
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

        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            webhook_url,
            cpu_threshold_bp: cpu_threshold * 100,
            mem_threshold_pct: mem_threshold,
        }
    }

    pub async fn check(&self, payload: &MetricsPayload) {
        let mut state = self.state.lock().await;
        let entry =
            state.entry(payload.agent_id.clone()).or_insert(AlertState { cpu_high_count: 0, mem_high_count: 0 });

        if let Some(ref cpu) = payload.cpu {
            if cpu.usage_total_bp > self.cpu_threshold_bp {
                entry.cpu_high_count += 1;
                if entry.cpu_high_count == 3 {
                    let msg = format!(
                        "CPU at {:.1}% (threshold: {}%)",
                        cpu.usage_total_bp as f32 / 100.0,
                        self.cpu_threshold_bp / 100
                    );
                    self.fire(&payload.agent_id, "cpu_high", &msg).await;
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
                    let msg = format!("Memory at {}% (threshold: {}%)", used_pct, self.mem_threshold_pct);
                    self.fire(&payload.agent_id, "mem_high", &msg).await;
                }
            } else {
                entry.mem_high_count = 0;
            }
        }
    }

    async fn fire(&self, agent_id: &str, alert_type: &str, message: &str) {
        warn!(agent_id, alert_type, message, "ALERT FIRED");

        if let Some(ref url) = self.webhook_url {
            let body = format_webhook_body(url, agent_id, alert_type, message);
            let url = url.clone();
            tokio::spawn(async move {
                let _ = post_webhook(&url, &body).await;
            });
        }
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
