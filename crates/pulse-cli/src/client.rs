use anyhow::Result;
use serde::Deserialize;

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    token: Option<String>,
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

impl Client {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        Self { http: reqwest::Client::new(), base_url: base_url.trim_end_matches('/').to_string(), token }
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

    pub async fn exec_command(&self, agent_id: &str, command: &str) -> Result<serde_json::Value> {
        let mut req = self.http.post(format!("{}/api/agents/{agent_id}/exec", self.base_url));
        if let Some(ref t) = self.token {
            req = req.bearer_auth(t);
        }
        let body = serde_json::json!({"command": command});
        Ok(req.json(&body).send().await?.json().await?)
    }

    pub async fn get_rollout(&self) -> Result<serde_json::Value> {
        self.get("/api/updates/rollout").await
    }

    #[allow(dead_code)]
    pub async fn set_rollout(&self, version: &str, pct: u8) -> Result<serde_json::Value> {
        let mut req = self.http.post(format!("{}/api/updates/rollout", self.base_url));
        if let Some(ref t) = self.token {
            req = req.bearer_auth(t);
        }
        let body = serde_json::json!({"version": version, "rollout_pct": pct});
        Ok(req.json(&body).send().await?.json().await?)
    }
}
