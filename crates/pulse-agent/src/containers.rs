use anyhow::Result;
use pulse_proto::pb::ContainerMetrics;
use std::path::Path;
use tokio::fs;

/// Collect container stats via cgroup fs (works without Docker socket access)
pub async fn collect_containers() -> Result<Vec<ContainerMetrics>> {
    // cgroup v2: Docker
    if Path::new("/sys/fs/cgroup/system.slice").exists() {
        return collect_from_cgroup_v2().await;
    }
    // cgroup v1: Docker
    if Path::new("/sys/fs/cgroup/docker").exists() {
        return collect_from_cgroup_v1().await;
    }
    // Podman rootless (user slice)
    if let Ok(uid) = std::env::var("UID").or_else(|_| std::env::var("EUID")) {
        let podman_path = format!("/sys/fs/cgroup/user.slice/user-{uid}.slice/user@{uid}.service");
        if Path::new(&podman_path).exists() {
            return collect_from_cgroup_v2_path(&podman_path, "libpod-").await;
        }
    }
    Ok(Vec::new())
}

async fn collect_from_cgroup_v2() -> Result<Vec<ContainerMetrics>> {
    collect_from_cgroup_v2_path("/sys/fs/cgroup/system.slice", "docker-").await
}

async fn collect_from_cgroup_v2_path(base: &str, prefix: &str) -> Result<Vec<ContainerMetrics>> {
    let mut containers = Vec::new();

    let mut entries = tokio::fs::read_dir(base).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(prefix) || !name.ends_with(".scope") {
            continue;
        }
        let id = name.trim_start_matches(prefix).trim_end_matches(".scope");
        let short_id = &id[..12.min(id.len())];
        let dir = entry.path();

        let cpu_usage = read_u64(&dir.join("cpu.stat"), "usage_usec").await;
        let mem_usage = read_file_u64(&dir.join("memory.current")).await;
        let mem_limit = read_file_u64(&dir.join("memory.max")).await;

        containers.push(ContainerMetrics {
            id: short_id.into(),
            name: short_id.into(),
            image: String::new(),
            state: "running".into(),
            cpu_bp: 0, // delta needed for %, report raw for now
            mem_usage,
            mem_limit,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
        });
        let _ = cpu_usage; // reserved for delta calculation
    }
    Ok(containers)
}

async fn collect_from_cgroup_v1() -> Result<Vec<ContainerMetrics>> {
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

        let short_id = &name[..12];
        let mem_usage = read_file_u64(&dir.join("memory.usage_in_bytes")).await;
        let mem_limit = read_file_u64(&dir.join("memory.limit_in_bytes")).await;

        containers.push(ContainerMetrics {
            id: short_id.into(),
            name: short_id.into(),
            image: String::new(),
            state: "running".into(),
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
