use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub current_context: String,
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
