# Features

## Metrics Collected

### CPU

| Metric | Detail |
|--------|--------|
| Total usage (%) | Delta between `/proc/stat` readings, iowait counted as idle |
| Per-core usage | Same calculation, per individual CPU |
| Load average 1/5/15m | From `/proc/loadavg`, centipercent format (1.50 = 150) |

### Memory

| Metric | Detail |
|--------|--------|
| Total | MemTotal from `/proc/meminfo` |
| Used | Total - Available - ZFS ARC |
| Available | MemAvailable (or MemFree if unavailable) |
| Swap total/used | SwapTotal - SwapFree |
| Cgroup limit | `/sys/fs/cgroup/memory.max` (v2) or `memory.limit_in_bytes` (v1) |
| Cgroup usage | `/sys/fs/cgroup/memory.current` (v2) or `memory.usage_in_bytes` (v1) |
| ZFS ARC | Subtracted from "used" (reclaimable memory) |

### Disk

| Metric | Detail |
|--------|--------|
| Usage (total/used) | `statvfs` syscall per mount point (ext4, xfs, btrfs, zfs, f2fs) |
| I/O read bytes | Delta from `/proc/diskstats` (sectors * 512) |
| I/O write bytes | Delta from `/proc/diskstats` |
| I/O read ops | Delta of read operations |
| I/O write ops | Delta of write operations |
| Health (model, temp) | `/sys/block/*/device/model` + hwmon |

### Network

| Metric | Detail |
|--------|--------|
| RX bytes (per interval) | Delta from `/proc/net/dev` |
| TX bytes (per interval) | Delta from `/proc/net/dev` |
| RX/TX packets | Delta |
| RX/TX errors | Accumulated (counter semantic) |

### Docker / Podman

| Metric | Detail |
|--------|--------|
| Container memory usage | Cgroup v2: `memory.current` |
| Container memory limit | Cgroup v2: `memory.max` |
| Container ID + state | Cgroup path name parsing |
| Podman rootless | Detects `user.slice` with `libpod-` prefix |

### GPU (NVIDIA)

| Metric | Detail |
|--------|--------|
| Utilization (%) | `nvidia-smi --query-gpu` |
| Memory (used/total) | In bytes |
| Temperature | Celsius |
| Power draw | Watts |

### System

| Metric | Detail |
|--------|--------|
| Uptime | `/proc/uptime` in seconds |
| Processes total | `/proc/stat` field `processes` |
| Processes running | `/proc/stat` field `procs_running` |
| Top 5 processes | `/proc/[pid]/stat` — sorted by CPU time + RSS |
| Temperature | `/sys/class/thermal/thermal_zone*/temp` (millicelsius) |
| Battery | `/sys/class/power_supply/BAT0/capacity` (%) |

### SSH Sessions

| Metric | Detail |
|--------|--------|
| Logged user | Binary parsing of `/var/run/utmp` |
| Source IP | `ut_host` field from utmpx struct |
| Terminal (pts/N) | `ut_line` field |
| sshd PID | `ut_pid` field |

### TCP Connections (Lightweight Netflow)

| Metric | Detail |
|--------|--------|
| Local port | Ports with ESTABLISHED connections |
| Service | Auto-detect: ssh(22), http(80), https(443), postgres(5432), mysql(3306), redis(6379), mongodb(27017) |
| Active count | Number of active connections per port |
| Remote IPs | Unique connected IPs (max 50 per port) |
| Source | `/proc/net/tcp` + `/proc/net/tcp6` |
| Filter | Ignores localhost, ESTABLISHED only (state=01) |

---

## Remote Commands

### Exec

Execute any program on the agent with streaming output.

- Program + arguments + environment variables
- Custom working directory
- Configurable timeout (default 300s)
- Automatic kill on timeout
- stdout/stderr streamed in real-time via gRPC

### Package

Package management via apt-get.

- `install` — install packages
- `remove` — remove packages
- `update` — update package list

### File

Remote file operations.

- `read` — read file contents (returned via stream)
- `write` — create/overwrite file
- `append` — append to end of file

### Log Tail

Real-time log streaming.

- Equivalent to `tail -n N [-f] /path/to/log`
- Configurable initial line count
- Follow mode: continuous stream until timeout

---

## Alerting

### Thresholds

| Alert | Trigger | Config Variable |
|-------|---------|-----------------|
| CPU high | > 90% for 3 consecutive readings | `PULSE_ALERT_CPU_PCT` |
| Memory high | > 90% for 3 consecutive readings | `PULSE_ALERT_MEM_PCT` |
| Bandwidth high | > 100 MB/s for 3 consecutive readings | `PULSE_ALERT_BW_MBPS` |
| Agent offline | No metrics for > 45 seconds | Automatic |

### Notification Channels

| Channel | Auto-detection |
|---------|---------------|
| Telegram | URL contains `api.telegram.org` |
| Slack | URL contains `hooks.slack.com` |
| Discord | URL contains `discord.com/api/webhooks` |
| Generic | Any other URL (JSON POST) |

---

## Security

- Automatic TLS: certificates generated on first master boot
- CA pin in token: prevents MITM attacks during bootstrap
- Automatic client certs: issued on Register, used on subsequent boots
- Full mTLS: after first registration, all connections use mutual TLS
- Auth tokens: separate tokens for registration, admin API, and CLI downloads
- Systemd hardening: NoNewPrivileges, ProtectSystem=strict
- Zero unsafe: except 1 syscall (statvfs, documented)
- Zero vulnerabilities: `cargo audit` clean

---

## Auto-Update

### Flow

1. CI compiles and uploads binaries to master (`POST /api/updates/upload`)
2. Master serves binaries and controls rollout
3. Agents check every 5 minutes (`GET /api/updates/check`)
4. Download + SHA256 verify + atomic self-replace + systemd restart

### Rollout

| Config | Description |
|--------|-------------|
| `rollout_pct = 100` | All agents (default) |
| `rollout_pct = 10` | 10% first (canary) |
| `urgent = true` | No jitter, immediate update |

### Automatic Rollback

- Agent saves `.prev` before replacing binary
- Marker file counts boot attempts
- 3 crashes in a row: restores previous binary automatically

---

## Persistence

### SQLite (default)

- Zero config, zero dependencies
- Database created automatically on boot
- Retention worker deletes data older than `PULSE_RETENTION_DAYS` (default: 30)

### PostgreSQL / TimescaleDB (optional, via `DATABASE_URL`)

- Hypertables: automatic time-based partitioning
- Compression: chunks older than 7 days compressed (~90% size reduction)
- Retention: configurable
- Continuous aggregates: pre-computed hourly averages
- Native Grafana integration

---

## Operations

- Health endpoint: `GET :8080/health`
- Exponential backoff reconnect: 1s to 60s
- Offline buffer: ring buffer of 100 metrics survives disconnections
- Graceful shutdown: SIGINT/SIGTERM closes connections properly
- Automatic retention: background worker deletes old data
