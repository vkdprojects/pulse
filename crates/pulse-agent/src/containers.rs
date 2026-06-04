use anyhow::Result;
use pulse_proto::pb::ContainerMetrics;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const DOCKER_SOCK: &str = "/var/run/docker.sock";

#[derive(Default)]
struct DockerInfo {
    name: String,
    image: String,
    state: String,
}

/// Best-effort lookup map (short_id → metadata) via the Docker Engine socket.
/// Returns an empty map if the socket is missing or unreadable — the caller falls back to ID.
async fn fetch_docker_meta() -> HashMap<String, DockerInfo> {
    let mut map = HashMap::new();
    let Ok(mut stream) = UnixStream::connect(DOCKER_SOCK).await else {
        return map;
    };
    let req = b"GET /containers/json?all=1 HTTP/1.0\r\nHost: localhost\r\n\r\n";
    if stream.write_all(req).await.is_err() {
        return map;
    }
    let mut buf = Vec::with_capacity(8192);
    if stream.read_to_end(&mut buf).await.is_err() {
        return map;
    }
    let Some(idx) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
        return map;
    };
    let body = &buf[idx + 4..];
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) else {
        return map;
    };
    let Some(arr) = json.as_array() else { return map };
    for c in arr {
        let Some(id) = c.get("Id").and_then(|v| v.as_str()) else { continue };
        let short = id.get(..12).unwrap_or(id).to_string();
        let name = c
            .get("Names")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_start_matches('/')
            .to_string();
        let image = c.get("Image").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = c.get("State").and_then(|v| v.as_str()).unwrap_or("running").to_string();
        map.insert(short, DockerInfo { name, image, state });
    }
    map
}

/// Collect container stats via cgroup fs (works without Docker socket access)
pub async fn collect_containers() -> Result<Vec<ContainerMetrics>> {
    let meta = fetch_docker_meta().await;

    // cgroup v2: Docker
    if Path::new("/sys/fs/cgroup/system.slice").exists() {
        return collect_from_cgroup_v2(&meta).await;
    }
    // cgroup v1: Docker
    if Path::new("/sys/fs/cgroup/docker").exists() {
        return collect_from_cgroup_v1(&meta).await;
    }
    // Podman rootless (user slice)
    if let Ok(uid) = std::env::var("UID").or_else(|_| std::env::var("EUID")) {
        let podman_path = format!("/sys/fs/cgroup/user.slice/user-{uid}.slice/user@{uid}.service");
        if Path::new(&podman_path).exists() {
            return collect_from_cgroup_v2_path(&podman_path, "libpod-", &meta).await;
        }
    }
    Ok(Vec::new())
}

async fn collect_from_cgroup_v2(meta: &HashMap<String, DockerInfo>) -> Result<Vec<ContainerMetrics>> {
    collect_from_cgroup_v2_path("/sys/fs/cgroup/system.slice", "docker-", meta).await
}

async fn collect_from_cgroup_v2_path(
    base: &str,
    prefix: &str,
    meta: &HashMap<String, DockerInfo>,
) -> Result<Vec<ContainerMetrics>> {
    let mut containers = Vec::new();

    let mut entries = tokio::fs::read_dir(base).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(prefix) || !name.ends_with(".scope") {
            continue;
        }
        let id = name.trim_start_matches(prefix).trim_end_matches(".scope");
        let short_id = id.get(..12).unwrap_or(id).to_string();
        let dir = entry.path();

        let cpu_usage = read_u64(&dir.join("cpu.stat"), "usage_usec").await;
        let mem_usage = read_file_u64(&dir.join("memory.current")).await;
        let mem_limit = read_file_u64(&dir.join("memory.max")).await;

        let info = meta.get(&short_id);
        containers.push(ContainerMetrics {
            id: short_id.clone(),
            name: info.map(|i| i.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| short_id.clone()),
            image: info.map(|i| i.image.clone()).unwrap_or_default(),
            state: info.map(|i| i.state.clone()).unwrap_or_else(|| "running".into()),
            cpu_bp: 0,
            mem_usage,
            mem_limit,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
        });
        let _ = cpu_usage;
    }
    Ok(containers)
}

async fn collect_from_cgroup_v1(meta: &HashMap<String, DockerInfo>) -> Result<Vec<ContainerMetrics>> {
    let mut containers = Vec::new();
    let base = "/sys/fs/cgroup/docker";

    let mut entries = tokio::fs::read_dir(base).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.len() < 12 || name == "." || name == ".." {
            continue;
        }
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }

        let short_id = name[..12].to_string();
        let mem_usage = read_file_u64(&dir.join("memory.usage_in_bytes")).await;
        let mem_limit = read_file_u64(&dir.join("memory.limit_in_bytes")).await;

        let info = meta.get(&short_id);
        containers.push(ContainerMetrics {
            id: short_id.clone(),
            name: info.map(|i| i.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| short_id.clone()),
            image: info.map(|i| i.image.clone()).unwrap_or_default(),
            state: info.map(|i| i.state.clone()).unwrap_or_else(|| "running".into()),
            cpu_bp: 0,
            mem_usage,
            mem_limit,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
        });
    }
    Ok(containers)
}

async fn read_file_u64(path: &Path) -> u64 {
    fs::read_to_string(path).await.ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

async fn read_u64(path: &Path, key: &str) -> u64 {
    let content = fs::read_to_string(path).await.unwrap_or_default();
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            return rest.trim().parse().unwrap_or(0);
        }
    }
    0
}
