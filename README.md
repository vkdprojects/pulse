# Pulse

**Infrastructure monitoring that fits in your server's pocket.**

One binary. 2-5 MB RAM. Full telemetry. Remote execution. Auto-updates. Zero dependencies.

```
┌─────────────┐    gRPC/mTLS     ┌──────────────┐
│ pulse-agent │ ═══════════════▶ │ pulse-master │ ──▶ REST API :8080
│   (3.1 MB)  │ ◀═══════════════ │   (5.8 MB)   │ ──▶ SQLite embedded
└─────────────┘   commands        └──────────────┘     (zero config)
     × N agents                    auto-provisioning

┌─────────────┐
│   pulse     │  TUI/CLI for operators (2.0 MB)
│  (your mac) │  Install, monitor, update — all from terminal
└─────────────┘
```

---

## Getting Started

### Option A: TUI (recommended)

Install the `pulse` CLI on your machine and manage everything from there.

```bash
# macOS / Linux — no sudo needed
curl -sfL https://raw.githubusercontent.com/vkdprojects/pulse/main/dist/install-pulse.sh | \
  INSTALL_COMPONENT=cli sh -
```

Then run:

```bash
pulse
```

The TUI guides you through:
1. First-run setup (GitHub token for downloading binaries)
2. Installing a master on a remote server via SSH
3. Adding agents to your fleet
4. Monitoring everything in real-time

```
    ╭───╮
   ╭┤ ◉ ├╮   ┏━┓╻ ╻╻  ┏━┓┏━╸
   │╰─┬─╯│   ┣━┛┃ ┃┃  ┗━┓┣╸
   ╰──┼──╯   ╹  ┗━┛┗━╸┗━┛┗━╸
      ╵

  Pulse monitors your servers: CPU, memory, disk, network — real-time.
```

### Option B: Manual install (curl | sh)

```bash
# Master server
curl -sfL .../install-pulse.sh | sudo sh -

# Each monitored server
curl -sfL .../install-pulse.sh | \
  PULSE_MASTER=https://MASTER-IP:50051 PULSE_TOKEN=xxx sudo sh -
```

---

## How It Works

```
You (laptop)                    Master (central)                 Agents (servers)
────────────                    ────────────────                 ───────────────
pulse (TUI)  ──── REST :8080 ──── pulse-master ──── gRPC/mTLS ──── pulse-agent
                                       │                              │
                                  SQLite DB                     /proc, /sys, cgroup
                                  Auto-TLS                     nvidia-smi, Docker
                                  Alerting                     Remote exec
```

- **Master** runs on one server. Stores metrics, serves API, manages TLS certificates.
- **Agents** run on every server you want to monitor. 2-5 MB RAM, invisible footprint.
- **pulse** runs on your laptop. Connects to master API. Install, monitor, update.

---

## TUI Screens

### Fleet Overview
Real-time view of all your servers with inline CPU sparklines.

```
 ◉ pulse homelab  │  12 agents  ● 11  ○ 1

    HOST              CPU ─────────  CPU   MEM   UPTIME   VER
 ●  web-prod-01      ▂▃▅▇█▅▃▂▁▂    23%   67%   12d 4h   0.4.0
 ●  db-master        ▁▁▂▁▁▁▁▁▁▁    12%   89%    5d 1h   0.4.0
 ○  worker-03        ──────────      —     —    offline   0.3.0
 ●  cache-redis      ▁▁▁▁▁▁▁▁▁▁     3%   34%   42d 7h   0.4.0
```

### Agent Detail
Grafana-style dashboard: stat cards, braille charts, disk/network tables.

```
 ◉ pulse web-prod-01  ● ↑12d4h  0.4.0    [System] [Processes] [Containers] [GPU]

 ╭──────╮ ╭──────╮ ╭──────╮ ╭──────╮ ╭──────╮
 │23.4% │ │67.2% │ │ 1.23 │ │  0%  │ │  3   │
 │ CPU  │ │ MEM  │ │ LOAD │ │ SWAP │ │PROCS │
 ╰──────╯ ╰──────╯ ╰──────╯ ╰──────╯ ╰──────╯

 ╭── CPU % ──────────────────╮ ╭── MEM % ──────────────────╮
 │  ⣠⣤⣶⣿⣿⣶⣤⣀⠀⣤⣶⣿⣿⣶     │ │  ⣀⣤⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶⣶     │
 ╰───────────────────────────╯ ╰───────────────────────────╯
```

### SSH Installer
Install master or agents on remote servers without leaving the terminal.

```
 ◉ pulse Instalar Master via SSH

  Hosts from ~/.ssh/config:
  › pulse-master-vm    ubuntu@192.168.68.66:22  (codeatlas)
    pulse-agent-1-vm   ubuntu@192.168.68.68:22  (codeatlas)
    Manual...
```

Reads your `~/.ssh/config`, respects Keychain and ssh-agent. No passwords needed if your keys are configured.

---

## Auto-Update

Agents self-update without SSH access. Push a tag, agents update in 5 minutes.

```
git tag v0.5.0 && git push --tags
  → CI builds binaries
  → CI uploads to master
  → CI sets rollout target
  → Agents poll every 5 min
  → Download, SHA256 verify, atomic replace, restart
```

- Gradual rollout: 10% → 50% → 100%
- Automatic rollback: 3 crashes → restores previous binary
- Zero downtime: systemd restart takes < 1 second

---

## What It Collects

| Category | Metrics |
|----------|---------|
| CPU | Per-core usage, total %, load average (1/5/15m) |
| Memory | Used, available, swap, cgroup limits (v1+v2) |
| Disk | Usage per mount, I/O bytes/ops, health (model, temp) |
| Network | RX/TX bytes per interface, packets, errors |
| Docker | Per-container CPU, memory, state |
| GPU | NVIDIA: utilization, memory, temp, power |
| System | Uptime, processes, temperatures, battery |
| SSH | Active sessions (user, IP, terminal, PID) |
| TCP | Connections per port with remote IPs (netflow-lite) |

---

## REST API

Base: `http://master:8080` — Protected by `PULSE_ADMIN_TOKEN` for write operations.

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check |
| `GET /api/overview` | All agents with latest metrics |
| `GET /api/agents` | Agent list |
| `GET /api/agents/:id/metrics` | Last 20 readings |
| `GET /api/agents/:id/metrics/latest` | Full current snapshot |
| `GET /api/agents/:id/disks` | Disk usage |
| `GET /api/agents/:id/network` | Network interfaces |
| `GET /api/agents/:id/containers` | Docker containers |
| `GET /api/agents/:id/processes` | Top processes |
| `GET /api/agents/:id/gpu` | GPU stats |
| `GET /api/agents/:id/connections` | TCP connections |
| `PUT /api/agents/:id/name` | Rename agent |
| `DELETE /api/agents/:id` | Remove agent |
| `POST /api/agents/:id/exec` | Execute command |
| `GET /api/updates/check` | Check for updates |
| `GET /api/updates/download` | Download binary |
| `POST /api/updates/upload` | Upload binary (CI) |
| `POST /api/updates/rollout` | Set rollout config |
| `GET /api/alerts` | Alert history |

---

## Security

| Layer | How |
|-------|-----|
| Wire encryption | Mutual TLS (mTLS) on all gRPC traffic |
| CA pin | Token contains SHA256 of CA cert — prevents MITM |
| Auto-provisioning | Master issues client certs on registration |
| Auth tokens | Separate: register (agents), admin (operators) |
| Systemd hardening | NoNewPrivileges, ProtectSystem=strict |
| Binary verification | SHA256 check before applying updates |

---

## Architecture

```
crates/
├── pulse-proto/       Protobuf definitions (gRPC contract)
├── pulse-agent/       Monitoring daemon (3.1 MB)
│   ├── telemetry.rs   /proc + /sys + cgroup + nvidia-smi
│   ├── worker.rs      Remote execution (exec, file, package, log-tail)
│   └── updater.rs     Self-update (download, verify, replace, rollback)
├── pulse-master/      Central server (5.8 MB)
│   ├── service.rs     gRPC service + auto-provisioning
│   ├── storage.rs     SQLite (default) or Postgres (via DATABASE_URL)
│   ├── api.rs         REST API (axum)
│   ├── auth.rs        Bearer token middleware
│   ├── updates.rs     Binary distribution + rollout
│   ├── alerts.rs      Threshold alerting → webhooks
│   └── ca.rs          X.509 CA (certificate generation)
├── pulse-cli/         TUI for operators (2.0 MB, binary: "pulse")
│   ├── installer.rs   SSH-based provisioning (uses system ssh/scp)
│   ├── github.rs      GitHub Release API client
│   ├── ssh_config.rs  Parses ~/.ssh/config
│   └── screens/       Home, Fleet, Detail, Exec, Update, Provision, Setup
└── pulse-tests/       Integration tests (gRPC + mTLS scenarios)
```

---

## Numbers

| Metric | Value |
|--------|-------|
| Agent binary | 3.1 MB |
| Master binary | 5.8 MB |
| CLI binary | 2.0 MB |
| Agent RAM (idle) | 2-5 MB |
| Master RAM (idle) | ~5 MB |
| External dependencies | 0 |
| Install time | < 5 seconds |
| First metric after boot | ~1 second |
| Automated tests | 93 |
| Vulnerabilities | 0 |
| Unsafe code | 0 |

---

## Development

```bash
# Prerequisites: Rust 1.88+, protoc
brew install protobuf just   # macOS

# Build
cargo build --workspace

# Run quality checks
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Or use just:
just ci
```

---

## Configuration

### Master (env vars)
```
PULSE_LISTEN=0.0.0.0:50051
PULSE_TLS_DIR=/var/lib/pulse/certs
DATABASE_URL=postgres://...     # optional, default: SQLite
PULSE_ALERT_WEBHOOK=https://...
PULSE_ALERT_CPU_PCT=90
PULSE_ALERT_MEM_PCT=90
PULSE_RETENTION_DAYS=30
```

### Agent (TOML)
```toml
master_endpoint = "https://master:50051"
metrics_interval_ms = 10000
auth_token = "sha256::secret"
```

### CLI (~/.config/pulse/config.toml)
```toml
github_token = "ghp_..."
github_repo = "vkdprojects/pulse"
current_context = "production"

[[masters]]
name = "production"
url = "http://master:8080"
admin_token = "..."
```

---

## Documentation

See [docs/](./docs/) for detailed guides:
- [CLI Installation](./docs/cli-install.md)
- [Auto-Update System](./docs/auto-update.md)
- [API Reference](./docs/api.md)
- [Security & TLS](./docs/security.md)
- [Alerting](./docs/alerting.md)
- [Troubleshooting](./docs/troubleshooting.md)

---

MIT — see [LICENSE](LICENSE).
