use anyhow::Result;
use serde::Deserialize;

#[derive(Clone)]
pub struct Client {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: String,
    pub(crate) token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentOverview {
    pub id: String,
    pub hostname: String,
    pub status: String,
    pub cpu_pct: Option<f64>,
    pub mem_pct: Option<f64>,
    pub uptime_secs: Option<i64>,
    #[serde(default)]
    pub version: String,
    // Local sparkline history (not from API)
    #[serde(skip)]
    pub cpu_history: Vec<u64>,
}

impl AgentOverview {
    pub fn cpu_history_spark(&self) -> String {
        const CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        if self.cpu_history.is_empty() {
            return self
                .cpu_pct
                .map(|v| {
                    let idx = ((v / 100.0) * 7.0).round() as usize;
                    CHARS[idx.min(7)].to_string().repeat(10)
                })
                .unwrap_or_else(|| "──────────".into());
        }
        self.cpu_history
            .iter()
            .rev()
            .take(10)
            .map(|v| {
                let idx = ((*v as f64 / 100.0) * 7.0).round() as usize;
                CHARS[idx.min(7)]
            })
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentMetric {
    #[allow(dead_code)]
    pub time: String,
    pub cpu_pct: Option<f64>,
    pub mem_used_mb: Option<i64>,
    pub mem_total_mb: Option<i64>,
    pub uptime_secs: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub cpu_pct: Option<f64>,
    pub rss_mb: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiskInfo {
    pub mount_point: String,
    pub total_gb: Option<f64>,
    pub used_gb: Option<f64>,
    pub used_pct: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetInterface {
    pub name: String,
    pub rx_bytes: Option<i64>,
    pub tx_bytes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: Option<String>,
    pub state: Option<String>,
    pub cpu_pct: Option<f64>,
    pub mem_mb: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GpuInfo {
    pub index: i32,
    pub name: Option<String>,
    pub utilization_pct: Option<i32>,
    pub mem_used_mb: Option<i64>,
    pub mem_total_mb: Option<i64>,
    pub temp_c: Option<i32>,
    pub power_watts: Option<i32>,
}

impl Client {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        Self {
            http: reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        }
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let mut req = self.http.get(format!("{}{}", self.base_url, path));
        if let Some(ref t) = self.token {
            req = req.bearer_auth(t);
        }
        Ok(req.send().await?.json().await?)
    }

    pub async fn overview(&self) -> Result<Vec<AgentOverview>> {
        let v = self.get("/api/overview").await?;
        let agents = v.get("agents").cloned().unwrap_or_default();
        Ok(serde_json::from_value(agents).unwrap_or_default())
    }

    pub async fn agent_metrics(&self, id: &str) -> Result<Vec<AgentMetric>> {
        let v = self.get(&format!("/api/agents/{id}/metrics")).await?;
        let metrics = v.get("metrics").cloned().unwrap_or_default();
        Ok(serde_json::from_value(metrics).unwrap_or_default())
    }

    pub async fn agent_processes(&self, id: &str) -> Result<Vec<ProcessInfo>> {
        let v = self.get(&format!("/api/agents/{id}/processes")).await?;
        let procs = v.get("processes").cloned().unwrap_or_default();
        Ok(serde_json::from_value(procs).unwrap_or_default())
    }

    pub async fn agent_disks(&self, id: &str) -> Result<Vec<DiskInfo>> {
        let v = self.get(&format!("/api/agents/{id}/disks")).await?;
        let disks = v.get("disks").cloned().unwrap_or_default();
        Ok(serde_json::from_value(disks).unwrap_or_default())
    }

    pub async fn agent_network(&self, id: &str) -> Result<Vec<NetInterface>> {
        let v = self.get(&format!("/api/agents/{id}/network")).await?;
        let ifaces = v.get("interfaces").cloned().unwrap_or_default();
        Ok(serde_json::from_value(ifaces).unwrap_or_default())
    }

    pub async fn agent_containers(&self, id: &str) -> Result<Vec<ContainerInfo>> {
        let v = self.get(&format!("/api/agents/{id}/containers")).await?;
        let containers = v.get("containers").cloned().unwrap_or_default();
        Ok(serde_json::from_value(containers).unwrap_or_default())
    }

    pub async fn agent_gpu(&self, id: &str) -> Result<Vec<GpuInfo>> {
        let v = self.get(&format!("/api/agents/{id}/gpu")).await?;
        let gpus = v.get("gpus").cloned().unwrap_or_default();
        Ok(serde_json::from_value(gpus).unwrap_or_default())
    }

    pub async fn agent_latest(&self, id: &str) -> Result<serde_json::Value> {
        let v = self.get(&format!("/api/agents/{id}/metrics/latest")).await?;
        Ok(v)
    }

    pub async fn health_check(&self) -> bool {
        self.get("/health").await.is_ok()
    }

    pub async fn register_token(&self) -> Result<String> {
        let v = self.get("/api/register-token").await?;
        Ok(v.get("register_token").and_then(|t| t.as_str()).unwrap_or("").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_overview_sparkline_empty_history() {
        let agent = AgentOverview {
            id: "a".into(),
            hostname: "test".into(),
            status: "online".into(),
            cpu_pct: Some(50.0),
            mem_pct: None,
            uptime_secs: None,
            version: "0.1.0".into(),
            cpu_history: vec![],
        };
        let spark = agent.cpu_history_spark();
        // With cpu_pct=50% and no history, shows repeated mid-level chars
        assert_eq!(spark.chars().count(), 10);
    }

    #[test]
    fn test_agent_overview_sparkline_with_history() {
        let agent = AgentOverview {
            id: "a".into(),
            hostname: "test".into(),
            status: "online".into(),
            cpu_pct: None,
            mem_pct: None,
            uptime_secs: None,
            version: "".into(),
            cpu_history: vec![10, 30, 50, 70, 90],
        };
        let spark = agent.cpu_history_spark();
        assert!(!spark.is_empty());
        // Should show increasing levels
    }

    #[test]
    fn test_agent_overview_sparkline_no_data() {
        let agent = AgentOverview {
            id: "a".into(),
            hostname: "test".into(),
            status: "offline".into(),
            cpu_pct: None,
            mem_pct: None,
            uptime_secs: None,
            version: "".into(),
            cpu_history: vec![],
        };
        let spark = agent.cpu_history_spark();
        assert_eq!(spark, "──────────");
    }
}
