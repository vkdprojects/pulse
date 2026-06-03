use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_endpoint")]
    pub master_endpoint: String,

    #[serde(default = "default_interval")]
    pub metrics_interval_ms: u64,

    #[serde(default)]
    pub labels: Vec<String>,

    #[serde(default)]
    pub tls: TlsConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct TlsConfig {
    pub ca_cert: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
}

impl Config {
    pub fn load(path: Option<&str>) -> Result<Self> {
        let path = path.unwrap_or("/etc/pulse-agent.toml");
        if Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn tls_enabled(&self) -> bool {
        self.tls.ca_cert.is_some() && self.tls.client_cert.is_some() && self.tls.client_key.is_some()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            master_endpoint: default_endpoint(),
            metrics_interval_ms: default_interval(),
            labels: Vec::new(),
            tls: TlsConfig::default(),
        }
    }
}

fn default_endpoint() -> String {
    "http://[::1]:50051".into()
}

fn default_interval() -> u64 {
    10_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.master_endpoint, "http://[::1]:50051");
        assert_eq!(cfg.metrics_interval_ms, 10_000);
        assert!(!cfg.tls_enabled());
    }

    #[test]
    fn test_parse_toml() {
        let toml = r#"
master_endpoint = "https://master.example.com:50051"
metrics_interval_ms = 5000
labels = ["prod", "us-east-1"]

[tls]
ca_cert = "/etc/pulse/ca.pem"
client_cert = "/etc/pulse/agent.pem"
client_key = "/etc/pulse/agent-key.pem"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.master_endpoint, "https://master.example.com:50051");
        assert_eq!(cfg.metrics_interval_ms, 5000);
        assert_eq!(cfg.labels, vec!["prod", "us-east-1"]);
        assert!(cfg.tls_enabled());
    }
}
