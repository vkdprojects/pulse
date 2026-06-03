use anyhow::Result;
use pulse_proto::pb::{CpuMetrics, DiskMetrics, MemoryMetrics, NetMetrics, SystemMetrics};
use std::path::Path;
use tokio::fs;

pub async fn collect_cpu() -> Result<CpuMetrics> {
    let stat = fs::read_to_string("/proc/stat").await?;
    let mut usage_per_core = Vec::new();
    let mut total_bp = 0u32;

    for line in stat.lines() {
        if line.starts_with("cpu") {
            let parts: Vec<u64> = line.split_whitespace().skip(1).filter_map(|s| s.parse().ok()).collect();
            if parts.len() < 4 {
                continue;
            }
            let idle = parts[3];
            let total: u64 = parts.iter().sum();
            let busy_bp = if total > 0 { ((total - idle) * 10_000 / total) as u32 } else { 0 };

            if line.starts_with("cpu ") {
                total_bp = busy_bp;
            } else {
                usage_per_core.push(busy_bp);
            }
        }
    }

    let loadavg = fs::read_to_string("/proc/loadavg").await.unwrap_or_default();
    let loads: Vec<u32> =
        loadavg.split_whitespace().take(3).filter_map(|s| s.parse::<f32>().ok().map(|v| (v * 100.0) as u32)).collect();

    Ok(CpuMetrics {
        usage_per_core_bp: usage_per_core,
        usage_total_bp: total_bp,
        load_avg_1m_cp: loads.first().copied().unwrap_or(0),
        load_avg_5m_cp: loads.get(1).copied().unwrap_or(0),
        load_avg_15m_cp: loads.get(2).copied().unwrap_or(0),
    })
}

pub async fn collect_memory() -> Result<MemoryMetrics> {
    let meminfo = fs::read_to_string("/proc/meminfo").await?;
    let mut total = 0u64;
    let mut available = 0u64;
    let mut free = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    for line in meminfo.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().unwrap_or("");
        let val: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0) * 1024; // kB → bytes
        match key {
            "MemTotal:" => total = val,
            "MemAvailable:" => available = val,
            "MemFree:" => free = val,
            "SwapTotal:" => swap_total = val,
            "SwapFree:" => swap_free = val,
            _ => {}
        }
    }
    if available == 0 {
        available = free;
    }

    let (cgroup_limit, cgroup_usage) = collect_cgroup_memory().await;

    Ok(MemoryMetrics {
        total_bytes: total,
        used_bytes: total.saturating_sub(available),
        available_bytes: available,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_total.saturating_sub(swap_free),
        cgroup_limit_bytes: cgroup_limit,
        cgroup_usage_bytes: cgroup_usage,
    })
}

async fn collect_cgroup_memory() -> (u64, u64) {
    // cgroup v2
    if Path::new("/sys/fs/cgroup/memory.max").exists() {
        let limit = read_cgroup_value("/sys/fs/cgroup/memory.max").await;
        let usage = read_cgroup_value("/sys/fs/cgroup/memory.current").await;
        return (limit, usage);
    }
    // cgroup v1
    if Path::new("/sys/fs/cgroup/memory/memory.limit_in_bytes").exists() {
        let limit = read_cgroup_value("/sys/fs/cgroup/memory/memory.limit_in_bytes").await;
        let usage = read_cgroup_value("/sys/fs/cgroup/memory/memory.usage_in_bytes").await;
        return (limit, usage);
    }
    (0, 0)
}

async fn read_cgroup_value(path: &str) -> u64 {
    fs::read_to_string(path).await.ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

pub async fn collect_net() -> Result<Vec<NetMetrics>> {
    let content = fs::read_to_string("/proc/net/dev").await?;
    let mut metrics = Vec::new();

    for line in content.lines().skip(2) {
        let line = line.trim();
        let (iface, rest) = match line.split_once(':') {
            Some(v) => v,
            None => continue,
        };
        let iface = iface.trim().to_string();
        if iface == "lo" {
            continue;
        }

        let vals: Vec<u64> = rest.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        if vals.len() < 16 {
            continue;
        }

        metrics.push(NetMetrics {
            interface: iface,
            rx_bytes_sec: vals[0],
            rx_packets_sec: vals[1],
            rx_errors: vals[2],
            tx_bytes_sec: vals[8],
            tx_packets_sec: vals[9],
            tx_errors: vals[10],
        });
    }
    Ok(metrics)
}

pub async fn collect_disks() -> Result<Vec<DiskMetrics>> {
    let mounts = fs::read_to_string("/proc/mounts").await?;
    let mut disks = Vec::new();

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let mount = parts[1];
        let fstype = parts[2];

        if !matches!(fstype, "ext4" | "xfs" | "btrfs" | "zfs" | "f2fs") {
            continue;
        }

        // Use statfs via nix would be better, but we keep it zero-dep
        // For now report mount point only — full disk stats need statfs syscall
        disks.push(DiskMetrics {
            mount_point: mount.to_string(),
            total_bytes: 0,
            used_bytes: 0,
            read_bytes_sec: 0,
            write_bytes_sec: 0,
        });
    }
    Ok(disks)
}

pub async fn collect_system() -> Result<SystemMetrics> {
    let uptime = fs::read_to_string("/proc/uptime").await.unwrap_or_default();
    let uptime_secs = uptime.split_whitespace().next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0) as u64;

    let stat = fs::read_to_string("/proc/stat").await.unwrap_or_default();
    let mut procs_total = 0u32;
    let mut procs_running = 0u32;
    for line in stat.lines() {
        if line.starts_with("processes ") {
            procs_total = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if line.starts_with("procs_running ") {
            procs_running = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }

    Ok(SystemMetrics { uptime_secs, processes_total: procs_total, processes_running: procs_running })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_collect_cpu_on_linux() {
        if !Path::new("/proc/stat").exists() {
            return; // Skip on non-Linux
        }
        let cpu = collect_cpu().await.unwrap();
        assert!(cpu.usage_total_bp <= 10_000);
        assert!(!cpu.usage_per_core_bp.is_empty());
    }

    #[tokio::test]
    async fn test_collect_memory_on_linux() {
        if !Path::new("/proc/meminfo").exists() {
            return;
        }
        let mem = collect_memory().await.unwrap();
        assert!(mem.total_bytes > 0);
        assert!(mem.available_bytes <= mem.total_bytes);
    }

    #[tokio::test]
    async fn test_collect_net_on_linux() {
        if !Path::new("/proc/net/dev").exists() {
            return;
        }
        let net = collect_net().await.unwrap();
        // At minimum should not error, may have 0 interfaces besides lo
        let _ = net;
    }

    #[tokio::test]
    async fn test_collect_system_on_linux() {
        if !Path::new("/proc/uptime").exists() {
            return;
        }
        let sys = collect_system().await.unwrap();
        assert!(sys.uptime_secs > 0);
    }
}
