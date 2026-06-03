use anyhow::Result;
use pulse_proto::pb::{
    CpuMetrics, DiskIoMetrics, DiskMetrics, MemoryMetrics, NetMetrics, ProcessInfo, SystemMetrics, TemperatureMetrics,
};
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

    // ZFS ARC: reclaimable memory counted as "used" but is actually free
    let arc_size = read_zfs_arc_size().await;
    let used = total.saturating_sub(available).saturating_sub(arc_size);

    let (cgroup_limit, cgroup_usage) = collect_cgroup_memory().await;

    Ok(MemoryMetrics {
        total_bytes: total,
        used_bytes: used,
        available_bytes: available + arc_size,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_total.saturating_sub(swap_free),
        cgroup_limit_bytes: cgroup_limit,
        cgroup_usage_bytes: cgroup_usage,
    })
}

async fn read_zfs_arc_size() -> u64 {
    let Ok(content) = fs::read_to_string("/proc/spl/kstat/zfs/arcstats").await else { return 0 };
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.first() == Some(&"size") && parts.len() >= 3 {
            return parts[2].parse().unwrap_or(0);
        }
    }
    0
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

use std::collections::HashMap as StdHashMap;
use std::sync::Mutex as StdMutex;

type CounterMap = StdMutex<StdHashMap<String, (u64, u64, u64, u64)>>;

static NET_PREV: std::sync::LazyLock<CounterMap> = std::sync::LazyLock::new(|| StdMutex::new(StdHashMap::new()));

pub async fn collect_net() -> Result<Vec<NetMetrics>> {
    let content = fs::read_to_string("/proc/net/dev").await?;
    let mut metrics = Vec::new();
    let mut prev = NET_PREV.lock().unwrap();

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

        let (rx_bytes, rx_packets, tx_bytes, tx_packets) = (vals[0], vals[1], vals[8], vals[9]);

        // Calculate delta from previous reading
        let (drx, drxp, dtx, dtxp) = if let Some(&(prx, prxp, ptx, ptxp)) = prev.get(&iface) {
            (
                rx_bytes.saturating_sub(prx),
                rx_packets.saturating_sub(prxp),
                tx_bytes.saturating_sub(ptx),
                tx_packets.saturating_sub(ptxp),
            )
        } else {
            (0, 0, 0, 0) // First reading: report 0
        };

        prev.insert(iface.clone(), (rx_bytes, rx_packets, tx_bytes, tx_packets));

        metrics.push(NetMetrics {
            interface: iface,
            rx_bytes_sec: drx,
            rx_packets_sec: drxp,
            rx_errors: vals[2],
            tx_bytes_sec: dtx,
            tx_packets_sec: dtxp,
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

        let (total, used) = statvfs_bytes(mount);
        disks.push(DiskMetrics {
            mount_point: mount.to_string(),
            total_bytes: total,
            used_bytes: used,
            read_bytes_sec: 0,
            write_bytes_sec: 0,
        });
    }
    Ok(disks)
}

fn statvfs_bytes(path: &str) -> (u64, u64) {
    use std::ffi::CString;
    let Ok(c_path) = CString::new(path) else { return (0, 0) };
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            let block_size = stat.f_frsize as u64;
            let total = (stat.f_blocks as u64) * block_size;
            let free = (stat.f_bfree as u64) * block_size;
            (total, total.saturating_sub(free))
        } else {
            (0, 0)
        }
    }
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

static DISK_PREV: std::sync::LazyLock<CounterMap> = std::sync::LazyLock::new(|| StdMutex::new(StdHashMap::new()));

pub async fn collect_disk_io() -> Result<Vec<DiskIoMetrics>> {
    let content = fs::read_to_string("/proc/diskstats").await?;
    let mut metrics = Vec::new();
    let mut prev = DISK_PREV.lock().unwrap();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 14 {
            continue;
        }
        let device = parts[2];
        if device.chars().last().is_some_and(|c| c.is_ascii_digit())
            && !device.contains("nvme")
            && !device.starts_with("dm-")
        {
            continue;
        }
        if device.starts_with("loop") || device.starts_with("ram") {
            continue;
        }

        let read_ops: u64 = parts[3].parse().unwrap_or(0);
        let read_sectors: u64 = parts[5].parse().unwrap_or(0);
        let write_ops: u64 = parts[7].parse().unwrap_or(0);
        let write_sectors: u64 = parts[9].parse().unwrap_or(0);

        let rb = read_sectors * 512;
        let wb = write_sectors * 512;

        let (drb, dwb, dro, dwo) = if let Some(&(prb, pwb, pro, pwo)) = prev.get(device) {
            (
                rb.saturating_sub(prb),
                wb.saturating_sub(pwb),
                read_ops.saturating_sub(pro),
                write_ops.saturating_sub(pwo),
            )
        } else {
            (0, 0, 0, 0)
        };

        prev.insert(device.to_string(), (rb, wb, read_ops, write_ops));

        metrics.push(DiskIoMetrics {
            device: device.into(),
            read_bytes: drb,
            write_bytes: dwb,
            read_ops: dro,
            write_ops: dwo,
        });
    }
    Ok(metrics)
}

pub async fn collect_top_processes() -> Result<Vec<ProcessInfo>> {
    let mut procs: Vec<(u32, String, u64, u64)> = Vec::new();
    let mut entries = fs::read_dir("/proc").await?;

    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let pid: u32 = match name.to_string_lossy().parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let stat_path = format!("/proc/{pid}/stat");
        let Ok(stat) = fs::read_to_string(&stat_path).await else { continue };

        // Parse: pid (comm) state ... field[13]=utime field[14]=stime field[23]=rss
        let comm_start = stat.find('(').unwrap_or(0);
        let comm_end = stat.rfind(')').unwrap_or(0);
        if comm_end <= comm_start {
            continue;
        }
        let comm = stat[comm_start + 1..comm_end].to_string();
        let rest: Vec<&str> = stat[comm_end + 2..].split_whitespace().collect();
        if rest.len() < 22 {
            continue;
        }

        let utime: u64 = rest[11].parse().unwrap_or(0);
        let stime: u64 = rest[12].parse().unwrap_or(0);
        let rss_pages: u64 = rest[21].parse().unwrap_or(0);

        procs.push((pid, comm, utime + stime, rss_pages * 4096));
    }

    // Sort by CPU time descending, take top 5
    procs.sort_by(|a, b| b.2.cmp(&a.2));
    procs.truncate(5);

    Ok(procs
        .into_iter()
        .map(|(pid, name, _cpu_ticks, rss)| ProcessInfo { pid, name, cpu_bp: 0, rss_bytes: rss })
        .collect())
}

pub async fn collect_temperatures() -> Result<Vec<TemperatureMetrics>> {
    let mut temps = Vec::new();
    let base = Path::new("/sys/class/thermal");
    if !base.exists() {
        return Ok(temps);
    }

    let mut entries = fs::read_dir(base).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let temp_path = entry.path().join("temp");
        let type_path = entry.path().join("type");

        let zone = fs::read_to_string(&type_path).await.unwrap_or(name).trim().to_string();
        let temp_mc: i32 = fs::read_to_string(&temp_path).await.ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);

        if temp_mc > 0 {
            temps.push(TemperatureMetrics { zone, temp_mc });
        }
    }
    Ok(temps)
}

pub async fn collect_gpus() -> Result<Vec<pulse_proto::pb::GpuMetrics>> {
    // Try nvidia-smi
    let output = tokio::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .await;

    let Ok(output) = output else { return Ok(Vec::new()) };
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() < 7 {
            continue;
        }
        gpus.push(pulse_proto::pb::GpuMetrics {
            index: parts[0].parse().unwrap_or(0),
            name: parts[1].into(),
            utilization_pct: parts[2].parse().unwrap_or(0),
            mem_used: parts[3].parse::<u64>().unwrap_or(0) * 1_048_576,
            mem_total: parts[4].parse::<u64>().unwrap_or(0) * 1_048_576,
            temp_c: parts[5].parse().unwrap_or(0),
            power_watts: parts[6].parse::<f32>().unwrap_or(0.0) as u32,
        });
    }
    Ok(gpus)
}

pub async fn collect_disk_health() -> Result<Vec<pulse_proto::pb::DiskHealth>> {
    let mut health = Vec::new();
    let block_dir = Path::new("/sys/block");
    if !block_dir.exists() {
        return Ok(health);
    }

    let mut entries = fs::read_dir(block_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
            continue;
        }

        let dev_path = entry.path().join("device");
        if !dev_path.exists() {
            continue;
        }

        let model = fs::read_to_string(dev_path.join("model")).await.unwrap_or_default().trim().to_string();

        // Try to read SMART-like attributes from sysfs
        let temp_path = entry.path().join("device/hwmon");
        let temp_c = if temp_path.exists() { read_hwmon_temp(&temp_path).await } else { 0 };

        health.push(pulse_proto::pb::DiskHealth {
            device: name,
            model,
            temp_c,
            power_on_hours: 0, // needs smartctl for this
            healthy: true,     // basic: exists = healthy
        });
    }
    Ok(health)
}

async fn read_hwmon_temp(hwmon_dir: &Path) -> u32 {
    let Ok(mut entries) = fs::read_dir(hwmon_dir).await else { return 0 };
    let Some(Ok(entry)) = entries.next_entry().await.transpose() else { return 0 };
    let temp_file = entry.path().join("temp1_input");
    fs::read_to_string(&temp_file).await.ok().and_then(|v| v.trim().parse::<u32>().ok()).unwrap_or(0) / 1000
}

pub async fn collect_battery() -> u32 {
    let path = Path::new("/sys/class/power_supply/BAT0/capacity");
    fs::read_to_string(path).await.ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
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
