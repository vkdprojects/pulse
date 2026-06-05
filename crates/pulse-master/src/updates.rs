use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ring::digest;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::api::AppState;

#[derive(Clone)]
pub struct UpdateState {
    pub releases_dir: PathBuf,
    pub config: Arc<RwLock<RolloutConfig>>,
    pub sha_cache: Arc<RwLock<std::collections::HashMap<String, (String, u64)>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutConfig {
    pub target_version: Option<String>,
    pub rollout_pct: u8,
    pub urgent: bool,
}

impl Default for RolloutConfig {
    fn default() -> Self {
        Self { target_version: None, rollout_pct: 100, urgent: false }
    }
}

impl UpdateState {
    pub fn new(data_dir: &str) -> Self {
        let releases_dir = PathBuf::from(data_dir).join("releases");
        std::fs::create_dir_all(&releases_dir).ok();
        let config_path = PathBuf::from(data_dir).join("rollout.json");
        let config = if config_path.exists() {
            std::fs::read_to_string(&config_path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
        } else {
            RolloutConfig::default()
        };
        Self {
            releases_dir,
            config: Arc::new(RwLock::new(config)),
            sha_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn persist_config(&self, cfg: &RolloutConfig) {
        let path = self.releases_dir.parent().unwrap_or(Path::new(".")).join("rollout.json");
        if let Ok(json) = serde_json::to_string_pretty(cfg) {
            std::fs::write(path, json).ok();
        }
    }

    /// Get binary path for a given version + arch (sanitized)
    pub fn binary_path(&self, version: &str, arch: &str) -> PathBuf {
        let version = sanitize_path_component(version);
        let arch = sanitize_path_component(arch);
        self.releases_dir.join(format!("pulse-agent-{version}-linux-{arch}"))
    }

    /// Should this agent receive an update signal?
    pub fn should_signal(&self, agent_id: &str, agent_version: &str, config: &RolloutConfig) -> bool {
        let Some(ref target) = config.target_version else { return false };
        if agent_version == target {
            return false;
        }
        // Deterministic bucket: agent always in same rollout group
        let hash = digest::digest(&digest::SHA256, agent_id.as_bytes());
        let bucket = hash.as_ref()[0] as u8 % 100;
        bucket < config.rollout_pct
    }

    /// Get SHA256 + size from cache (or compute and cache on miss)
    pub async fn get_binary_sha(&self, version: &str, arch: &str) -> Option<(String, u64)> {
        let key = format!("{version}-{arch}");
        {
            let cache = self.sha_cache.read().await;
            if let Some(v) = cache.get(&key) {
                return Some(v.clone());
            }
        }
        let path = self.binary_path(version, arch);
        let data = tokio::fs::read(&path).await.ok()?;
        let size = data.len() as u64;
        let hash = digest::digest(&digest::SHA256, &data);
        use std::fmt::Write;
        let mut sha = String::with_capacity(64);
        for b in hash.as_ref() {
            write!(sha, "{:02x}", b).unwrap();
        }
        self.sha_cache.write().await.insert(key, (sha.clone(), size));
        Some((sha, size))
    }
}

// ─── Endpoints ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CheckQuery {
    pub arch: String,
    pub version: String,
}

pub async fn check_update(State(state): State<AppState>, Query(q): Query<CheckQuery>) -> Json<Value> {
    let update_state = &state.update_state;
    let config = update_state.config.read().await;
    let Some(ref target) = config.target_version else {
        return Json(json!({"available": false}));
    };
    if q.version == *target {
        return Json(json!({"available": false}));
    }
    let target = target.clone();
    drop(config);
    let Some((sha, size)) = update_state.get_binary_sha(&target, &q.arch).await else {
        return Json(json!({"available": false, "reason": "binary not found for arch"}));
    };
    let config = update_state.config.read().await;
    Json(json!({
        "available": true,
        "version": target,
        "sha256": sha,
        "size_bytes": size,
        "urgent": config.urgent,
    }))
}

#[derive(Deserialize)]
pub struct DownloadQuery {
    pub version: String,
    pub arch: String,
}

pub async fn download_binary(State(state): State<AppState>, Query(q): Query<DownloadQuery>) -> Response {
    let path = state.update_state.binary_path(&q.version, &q.arch);
    if !path.exists() {
        return (StatusCode::NOT_FOUND, "binary not found").into_response();
    }
    match tokio::fs::read(&path).await {
        Ok(data) => {
            let headers = [
                (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
                (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"pulse-agent\""),
            ];
            (headers, data).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn upload_binary(State(state): State<AppState>, mut multipart: Multipart) -> Response {
    let mut version = String::new();
    let mut arch = String::new();
    let mut binary_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().unwrap_or_default() {
            "version" => version = field.text().await.unwrap_or_default(),
            "arch" => arch = field.text().await.unwrap_or_default(),
            "binary" => binary_data = field.bytes().await.ok().map(|b| b.to_vec()),
            _ => {}
        }
    }

    if version.is_empty() || arch.is_empty() {
        return (StatusCode::BAD_REQUEST, "version and arch required").into_response();
    }
    let Some(data) = binary_data else {
        return (StatusCode::BAD_REQUEST, "binary field required").into_response();
    };

    let path = state.update_state.binary_path(&version, &arch);
    match tokio::fs::write(&path, &data).await {
        Ok(()) => {
            let hash = digest::digest(&digest::SHA256, &data);
            use std::fmt::Write;
            let mut sha = String::with_capacity(64);
            for b in hash.as_ref() {
                write!(sha, "{:02x}", b).unwrap();
            }
            let size = data.len();
            let key = format!("{version}-{arch}");
            state.update_state.sha_cache.write().await.insert(key, (sha.clone(), size as u64));
            info!(version, arch, size, sha = %sha, "binary uploaded");
            Json(json!({"ok": true, "version": version, "arch": arch, "sha256": sha, "size": size})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct RolloutRequest {
    pub version: Option<String>,
    pub rollout_pct: Option<u8>,
    pub urgent: Option<bool>,
}

pub async fn set_rollout(State(state): State<AppState>, Json(req): Json<RolloutRequest>) -> Json<Value> {
    let mut config = state.update_state.config.write().await;
    if let Some(v) = req.version {
        config.target_version = Some(v);
    }
    if let Some(p) = req.rollout_pct {
        config.rollout_pct = p.min(100);
    }
    if let Some(u) = req.urgent {
        config.urgent = u;
    }
    state.update_state.persist_config(&config);
    info!(target = ?config.target_version, pct = config.rollout_pct, urgent = config.urgent, "rollout config updated");
    Json(json!({"ok": true, "config": *config}))
}

pub async fn get_rollout(State(state): State<AppState>) -> Json<Value> {
    let config = state.update_state.config.read().await;
    Json(json!({"config": *config}))
}

#[cfg(test)]
fn compute_sha256(path: &Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    let hash = digest::digest(&digest::SHA256, &data);
    use std::fmt::Write;
    let mut s = String::with_capacity(64);
    for b in hash.as_ref() {
        write!(s, "{:02x}", b).unwrap();
    }
    Some(s)
}

/// Sanitize path component: only allow [a-zA-Z0-9._-]
fn sanitize_path_component(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("v0.3.0"), "v0.3.0");
        assert_eq!(sanitize_path_component("x86_64"), "x86_64");
        assert_eq!(sanitize_path_component("../../etc/passwd"), "....etcpasswd");
        assert_eq!(sanitize_path_component("v1.0/../../root"), "v1.0....root");
        assert_eq!(sanitize_path_component("aarch64"), "aarch64");
    }

    #[test]
    fn test_binary_path_no_traversal() {
        let state = UpdateState::new("/tmp/pulse-test-updates");
        let path = state.binary_path("../../etc/passwd", "x86_64");
        assert!(!path.to_string_lossy().contains("etc/passwd"));
        assert!(path.starts_with("/tmp/pulse-test-updates/releases/"));
        // Cleanup
        std::fs::remove_dir_all("/tmp/pulse-test-updates").ok();
    }

    #[test]
    fn test_binary_path_normal() {
        let state = UpdateState::new("/tmp/pulse-test-updates2");
        let path = state.binary_path("v0.3.0", "aarch64");
        assert_eq!(path, PathBuf::from("/tmp/pulse-test-updates2/releases/pulse-agent-v0.3.0-linux-aarch64"));
        std::fs::remove_dir_all("/tmp/pulse-test-updates2").ok();
    }

    #[test]
    fn test_should_signal_no_target() {
        let state = UpdateState::new("/tmp/pulse-test-signal");
        let config = RolloutConfig::default(); // target_version = None
        assert!(!state.should_signal("agent-001", "0.1.0", &config));
        std::fs::remove_dir_all("/tmp/pulse-test-signal").ok();
    }

    #[test]
    fn test_should_signal_already_updated() {
        let state = UpdateState::new("/tmp/pulse-test-signal2");
        let config = RolloutConfig { target_version: Some("0.3.0".into()), rollout_pct: 100, urgent: false };
        assert!(!state.should_signal("agent-001", "0.3.0", &config));
        std::fs::remove_dir_all("/tmp/pulse-test-signal2").ok();
    }

    #[test]
    fn test_should_signal_needs_update_100pct() {
        let state = UpdateState::new("/tmp/pulse-test-signal3");
        let config = RolloutConfig { target_version: Some("0.3.0".into()), rollout_pct: 100, urgent: false };
        // 100% rollout → all agents should be signaled
        assert!(state.should_signal("agent-001", "0.2.0", &config));
        assert!(state.should_signal("agent-002", "0.2.0", &config));
        assert!(state.should_signal("agent-999", "0.1.0", &config));
        std::fs::remove_dir_all("/tmp/pulse-test-signal3").ok();
    }

    #[test]
    fn test_should_signal_rollout_0pct() {
        let state = UpdateState::new("/tmp/pulse-test-signal4");
        let config = RolloutConfig { target_version: Some("0.3.0".into()), rollout_pct: 0, urgent: false };
        // 0% rollout → no agents
        assert!(!state.should_signal("agent-001", "0.2.0", &config));
        assert!(!state.should_signal("agent-002", "0.2.0", &config));
        std::fs::remove_dir_all("/tmp/pulse-test-signal4").ok();
    }

    #[test]
    fn test_should_signal_deterministic() {
        let state = UpdateState::new("/tmp/pulse-test-signal5");
        let config = RolloutConfig { target_version: Some("0.3.0".into()), rollout_pct: 50, urgent: false };
        // Same agent always gets same result (deterministic)
        let r1 = state.should_signal("agent-001", "0.2.0", &config);
        let r2 = state.should_signal("agent-001", "0.2.0", &config);
        assert_eq!(r1, r2);
        std::fs::remove_dir_all("/tmp/pulse-test-signal5").ok();
    }

    #[test]
    fn test_rollout_config_default() {
        let cfg = RolloutConfig::default();
        assert_eq!(cfg.target_version, None);
        assert_eq!(cfg.rollout_pct, 100);
        assert!(!cfg.urgent);
    }

    #[test]
    fn test_compute_sha256() {
        let tmp = std::env::temp_dir().join("pulse_test_sha256");
        std::fs::write(&tmp, b"hello world").unwrap();
        let sha = compute_sha256(&tmp).unwrap();
        // Known SHA256 of "hello world"
        assert_eq!(sha, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_compute_sha256_nonexistent() {
        let result = compute_sha256(Path::new("/tmp/nonexistent_pulse_file_xyz"));
        assert!(result.is_none());
    }
}
