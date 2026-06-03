# Vixpi Pulse

Ultra-lightweight infrastructure monitoring agent. Streams hardware telemetry over gRPC with mutual TLS, persists to TimescaleDB.

**Agent binary: 1.9 MB** | **Target: 2-5 MB RAM idle** | **Zero dependencies at runtime**

## Architecture

```
┌─────────────┐                         ┌──────────────┐         ┌─────────────┐
│ pulse-agent │──── gRPC/mTLS ─────────▶│ pulse-master │────────▶│ TimescaleDB │
│  (per host) │◀── commands ────────────│  (central)   │         │  (metrics)  │
└─────────────┘                         └──────────────┘         └─────────────┘
     × N agents                              CA auto-provisioning
```

## Quick Start (Docker — recommended for validation)

Run the full production-like stack with 3 agents, mTLS, and TimescaleDB:

```bash
# Prerequisites: Docker

# Run full end-to-end test (builds, starts 3 agents + master + DB, validates)
./tests/e2e-docker.sh

# Or run interactively to inspect:
docker compose up --build
# Then in another terminal:
docker compose exec db psql -U pulse -c "SELECT * FROM agents;"
docker compose exec db psql -U pulse -c "SELECT time, agent_id, cpu_total_bp/100.0 as cpu_pct FROM metrics ORDER BY time DESC LIMIT 10;"
```

## Local Development

```bash
# Prerequisites: Rust 1.88+, protoc, Docker
brew install protobuf just  # macOS

# 1. Start TimescaleDB
just dev-up

# 2. Run master (terminal 1)
just dev-master

# 3. Run agent (terminal 2)
just dev-agent

# 4. Query the database
docker compose -f docker-compose.dev.yml exec db psql -U pulse

# 5. Stop everything
just dev-down
```

## What Gets Validated in E2E

The `./tests/e2e-docker.sh` script verifies:

| Check | What it proves |
|---|---|
| ≥3 agents in DB | Registration + auto-provisioning works |
| Metrics rows persisted | Stream → TimescaleDB pipeline works |
| CPU data present | `/proc/stat` parsing works on Linux |
| Memory data present | `/proc/meminfo` parsing works on Linux |
| mTLS connection | CA + certs + mutual validation works |
| Multiple unique agents | Concurrency + per-agent isolation works |
| Hypertables active | TimescaleDB schema + compression configured |

## Configuration

### Agent (`/etc/pulse-agent.toml`)

```toml
master_endpoint = "https://master.example.com:50051"
metrics_interval_ms = 10000
labels = ["prod", "us-east-1"]

[tls]
ca_cert = "/etc/pulse/ca.pem"
client_cert = "/etc/pulse/agent.pem"
client_key = "/etc/pulse/agent-key.pem"
domain_name = "master.example.com"
```

### Master (environment variables)

| Variable | Default | Description |
|---|---|---|
| `PULSE_LISTEN` | `[::1]:50051` | Listen address |
| `PULSE_TLS_DIR` | _(none)_ | Dir with `ca.pem`, `server.pem`, `server-key.pem` |
| `DATABASE_URL` | _(none)_ | PostgreSQL/TimescaleDB connection string |
| `RUST_LOG` | `info` | Log level filter |

### Certificate Generation

```bash
cargo run -p pulse-master -- gen-certs .certs
```

For production, use your own CA. The built-in CA is for development/testing.

## Telemetry Collected

| Metric | Source | Notes |
|---|---|---|
| CPU usage (per-core + total) | `/proc/stat` | Basis points (0-10000) |
| Load average (1/5/15m) | `/proc/loadavg` | Centipercent |
| Memory (total, used, available, swap) | `/proc/meminfo` | |
| Memory (cgroup limit/usage) | `/sys/fs/cgroup/memory.*` | v1 + v2 auto-detection |
| Disk (total, used per mount) | `statvfs` | ext4, xfs, btrfs, zfs, f2fs |
| Network (rx/tx bytes, packets, errors) | `/proc/net/dev` | Per interface |
| System (uptime, processes) | `/proc/uptime`, `/proc/stat` | |

## Remote Command Execution

The master can send commands to agents via the bidirectional `CommandChannel`:

- **Exec**: Run programs with args, env, working dir, timeout. stdout/stderr streamed real-time.
- **Package**: Install/remove/update packages (apt-get)
- **File**: Read/write/append files on the agent

## Database Schema (TimescaleDB)

```sql
-- Hypertable: auto-partitioned by time, compressed after 7 days, retained 90 days
SELECT * FROM metrics WHERE agent_id = 'agent-0001' AND time > now() - interval '1 hour';

-- Agents inventory
SELECT * FROM agents;
```

Policies:
- **Compression**: chunks >7 days compressed (~90% size reduction)
- **Retention**: chunks >90 days auto-dropped

## Development Commands

```bash
just --list           # All available commands

# Dev environment
just dev-up           # Start TimescaleDB
just dev-master       # Run master with DB + mTLS
just dev-agent        # Run agent
just dev-down         # Stop everything

# Quality
just ci               # fmt + clippy + test
just lint             # fmt + clippy only

# Docker
just docker-dev       # Full stack interactive
just docker-test      # Run e2e validation
just docker-down      # Stop Docker stack

# Release
just release-agent    # Optimized binary (1.9MB)
just release-master   # Optimized binary
```

## Testing

```bash
# Unit + integration tests (no Docker needed)
cargo test --workspace

# Full e2e with Docker (Linux telemetry + mTLS + TimescaleDB)
./tests/e2e-docker.sh
```

Tests cover:
- gRPC register (plaintext + mTLS)
- Metrics streaming
- mTLS rejection (no cert, wrong CA)
- Auto-provisioning of certificates
- Full pipeline: agent → master → TimescaleDB (Docker e2e)

## Deploy to Production

```bash
# Build static binary
rustup target add x86_64-unknown-linux-musl
cargo build --release -p pulse-agent --target x86_64-unknown-linux-musl

# Install on server
scp target/x86_64-unknown-linux-musl/release/pulse-agent server:/tmp/
scp dist/install.sh dist/pulse-agent.service server:/tmp/
ssh server 'sudo /tmp/install.sh /tmp/pulse-agent'
# Edit /etc/pulse-agent.toml, then:
ssh server 'sudo systemctl enable --now pulse-agent'
```

## Project Structure

```
crates/
├── pulse-proto/         # Protobuf definitions + generated code
├── pulse-agent/         # The monitoring daemon
│   └── src/
│       ├── main.rs      # Connect → register → stream metrics (reconnect + backoff)
│       ├── config.rs    # TOML config + defaults
│       ├── telemetry.rs # /proc + cgroup + statvfs parsing
│       └── worker.rs    # Command execution (exec/package/file)
├── pulse-master/        # The central server
│   └── src/
│       ├── main.rs      # Entry point, TLS, DB connect
│       ├── lib.rs       # Public API + cert generation
│       ├── service.rs   # gRPC implementation + auto-provisioning
│       ├── storage.rs   # TimescaleDB persistence (sqlx)
│       └── ca.rs        # X.509 CA (rcgen)
└── pulse-tests/         # Integration tests
docker-compose.yml       # Production-like: DB + master + 3 agents
docker-compose.dev.yml   # Dev: just the database
migrations/              # TimescaleDB schema
dist/                    # systemd unit + install script
tests/e2e-docker.sh      # End-to-end validation script
```

## License

MIT
