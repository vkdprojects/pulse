use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub current_context: String,
    #[serde(default)]
    pub github_token: Option<String>,
    #[serde(default)]
    pub github_repo: Option<String>,
    #[serde(default)]
    pub preferences: Preferences,
    #[serde(default)]
    pub masters: Vec<MasterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_refresh")]
    pub refresh_rate_secs: u64,
    #[serde(default = "default_theme")]
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub admin_token: Option<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self { refresh_rate_secs: default_refresh(), theme: default_theme() }
    }
}

fn default_refresh() -> u64 {
    5
}
fn default_theme() -> String {
    "default".into()
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn current_master(&self) -> Option<&MasterConfig> {
        self.masters.iter().find(|m| m.name == self.current_context)
    }

    pub fn is_first_run(&self) -> bool {
        self.github_token.is_none() && self.masters.is_empty()
    }

    pub fn repo(&self) -> &str {
        self.github_repo.as_deref().unwrap_or("vkdprojects/pulse")
    }

    pub fn add_master(&mut self, name: String, url: String, token: Option<String>) {
        self.masters.retain(|m| m.name != name);
        self.masters.push(MasterConfig { name: name.clone(), url, admin_token: token });
        if self.current_context.is_empty() {
            self.current_context = name;
        }
    }

    pub fn remove_master(&mut self, name: &str) {
        self.masters.retain(|m| m.name != name);
        if self.current_context == name {
            self.current_context = self.masters.first().map(|m| m.name.clone()).unwrap_or_default();
        }
    }
}

pub fn config_path() -> PathBuf {
    if let Ok(dir) = std::env::var("PULSE_CONFIG_DIR") {
        return PathBuf::from(dir).join("config.toml");
    }
    let home = dirs_home().join(".config").join("pulse");
    home.join("config.toml")
}

fn dirs_home() -> PathBuf {
    directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = Config::default();
        assert!(cfg.current_context.is_empty());
        assert!(cfg.masters.is_empty());
        assert_eq!(cfg.preferences.refresh_rate_secs, 5);
    }

    #[test]
    fn test_config_parse_toml() {
        let toml_str = r#"
current_context = "prod"

[preferences]
refresh_rate_secs = 10
theme = "dark"

[[masters]]
name = "prod"
url = "http://master:8080"
admin_token = "secret123"

[[masters]]
name = "staging"
url = "http://staging:8080"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.current_context, "prod");
        assert_eq!(cfg.masters.len(), 2);
        assert_eq!(cfg.masters[0].name, "prod");
        assert_eq!(cfg.masters[0].admin_token, Some("secret123".into()));
        assert_eq!(cfg.masters[1].admin_token, None);
        assert_eq!(cfg.preferences.refresh_rate_secs, 10);
    }

    #[test]
    fn test_add_master() {
        let mut cfg = Config::default();
        cfg.add_master("test".into(), "http://localhost:8080".into(), None);
        assert_eq!(cfg.masters.len(), 1);
        assert_eq!(cfg.current_context, "test"); // first added becomes current
    }

    #[test]
    fn test_add_master_deduplicates() {
        let mut cfg = Config::default();
        cfg.add_master("test".into(), "http://old:8080".into(), None);
        cfg.add_master("test".into(), "http://new:8080".into(), Some("tok".into()));
        assert_eq!(cfg.masters.len(), 1);
        assert_eq!(cfg.masters[0].url, "http://new:8080");
        assert_eq!(cfg.masters[0].admin_token, Some("tok".into()));
    }

    #[test]
    fn test_remove_master() {
        let mut cfg = Config::default();
        cfg.add_master("a".into(), "http://a:8080".into(), None);
        cfg.add_master("b".into(), "http://b:8080".into(), None);
        cfg.current_context = "a".into();
        cfg.remove_master("a");
        assert_eq!(cfg.masters.len(), 1);
        assert_eq!(cfg.current_context, "b"); // switches to remaining
    }

    #[test]
    fn test_current_master() {
        let mut cfg = Config::default();
        cfg.add_master("prod".into(), "http://prod:8080".into(), Some("tok".into()));
        cfg.current_context = "prod".into();
        let m = cfg.current_master().unwrap();
        assert_eq!(m.url, "http://prod:8080");
        assert_eq!(m.admin_token, Some("tok".into()));
    }

    #[test]
    fn test_current_master_none() {
        let cfg = Config::default();
        assert!(cfg.current_master().is_none());
    }

    /// Reproduces: user skipped setup (Esc), then added master via CLI arg.
    /// Config saved without github_token. Next launch must NOT show setup again.
    #[test]
    fn test_is_first_run_false_when_masters_exist_without_token() {
        let toml_str = r#"
current_context = "prod"

[preferences]
refresh_rate_secs = 5
theme = "default"

[[masters]]
name = "prod"
url = "http://prod:8080"
admin_token = "secret"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert!(cfg.github_token.is_none());
        assert!(!cfg.is_first_run(), "must not trigger setup when masters already configured");
    }

    #[test]
    fn test_is_first_run_true_when_empty() {
        let cfg = Config::default();
        assert!(cfg.is_first_run(), "fresh config with no token and no masters = first run");
    }
}
