# AGENTS.md

## Project Overview

Vixpi Pulse is an ultra-lightweight infrastructure monitoring agent (1.9 MB binary, 2-5 MB RAM) that streams hardware telemetry over gRPC with mutual TLS to a central master server. The master persists metrics in TimescaleDB and exposes a REST API. This is a Rust workspace (monorepo) with 4 crates.

## Tech Stack

- **Language:** Rust (edition 2024, MSRV 1.88)
- **Async runtime:** tokio (current_thread for agent, multi-thread for master)
- **RPC:** gRPC via tonic 0.14 + prost (protobuf)
- **TLS:** rustls + ring (mutual TLS)
- **Certificates:** rcgen (X.509 CA)
- **Database:** TimescaleDB (PostgreSQL 17 extension) via sqlx 0.8
- **REST API:** axum 0.8
- **Config:** TOML (serde) for agent, env vars for master
- **Logging:** tracing + tracing-subscriber
- **Build:** cargo, protoc (protobuf compiler)
- **CI:** GitHub Actions (fmt + clippy + test + cross-compile musl)
- **Containers:** Docker multi-stage (rust:1.88 builder → debian-slim runtime)

## Setup & Commands

```bash
# Prerequisites
brew install protobuf just   # macOS
# apt install protobuf-compiler  # Linux

# Install deps + build
cargo build --workspace

# Start dev environment (TimescaleDB in Docker)
just dev-up

# Run master (terminal 1) — connects to DB + mTLS
just dev-master

# Run agent (terminal 2) — connects to master
just dev-agent

# Run all quality checks
just ci   # = cargo fmt --check + clippy -D warnings + test

# Run e2e Docker test (full stack validation)
just docker-test

# Build release binaries
just release-agent    # → target/release/pulse-agent (1.9 MB)
just release-master
```

### Destructive commands (never run without explicit confirmation):
- `docker compose down -v` — deletes all data volumes
- `just dev-down` — stops and removes DB container + data
- `just clean` — removes target/ and .certs/

## Project Structure

```
crates/
├── pulse-proto/          # Protobuf definitions (shared contract)
│   ├── proto/pulse.proto # THE source of truth for all messages
│   ├── build.rs          # tonic-prost-build codegen
│   └── src/lib.rs        # Re-exports generated code
├── pulse-agent/          # The monitoring daemon (runs on each server)
│   └── src/
│       ├── main.rs       # Entry: connect → register → stream loop
│       ├── config.rs     # TOML config parsing
│       ├── telemetry.rs  # ALL /proc + /sys parsing (CPU, mem, disk, net, etc)
│       ├── containers.rs # Docker/Podman cgroup stats
│       └── worker.rs     # Remote command execution (exec/file/package/log-tail)
├── pulse-master/         # Central server
│   └── src/
│       ├── main.rs       # Entry: DB + TLS + gRPC + REST API
│       ├── lib.rs        # Public exports + gen_dev_certs()
│       ├── service.rs    # gRPC service implementation
│       ├── storage.rs    # TimescaleDB persistence (sqlx)
│       ├── api.rs        # REST API (axum) — /api/agents, /api/agents/:id/metrics
│       ├── alerts.rs     # Threshold alerting engine
│       ├── heartbeat.rs  # Background agent offline detection
│       └── ca.rs         # X.509 CA certificate generation
└── pulse-tests/          # Integration tests (gRPC + mTLS scenarios)
```

## Code Style & Conventions

```bash
cargo fmt --all           # Formatter (rustfmt, config in rustfmt.toml)
cargo clippy --workspace --all-targets -- -D warnings  # Linter (zero warnings policy)
```

- **Max line width:** 120 (rustfmt.toml)
- **No comments explaining WHAT** — only WHY (constraints, workarounds, kernel behavior)
- **No `unsafe`** except libc::statvfs (documented, necessary for disk stats)
- **Error handling:** `anyhow::Result` for app code, `tonic::Status` for gRPC handlers
- **Logging:** `tracing::{info, warn, error}` — never `println!`
- **No `unwrap()`** in production paths — use `?`, `.unwrap_or_default()`, or handle
- **Naming:** snake_case for files/functions, CamelCase for types, SCREAMING_SNAKE for statics

### Proto changes:
When modifying `pulse.proto`, you MUST also update:
1. `crates/pulse-tests/tests/integration.rs` (add new fields to MetricsPayload constructors)
2. `crates/pulse-agent/src/main.rs` (collect_snapshot function)
3. `crates/pulse-master/src/storage.rs` (if field should be persisted)

## Testing

```bash
# Full suite (27 tests)
cargo test --workspace

# Single crate
cargo test -p pulse-agent
cargo test -p pulse-master

# Specific test
cargo test -p pulse-agent -- test_exec_echo

# E2E Docker (Linux telemetry + mTLS + TimescaleDB, 7 checks)
./tests/e2e-docker.sh
```

- Tests live **inside the module** they test (`#[cfg(test)] mod tests`)
- Integration tests in `crates/pulse-tests/tests/integration.rs`
- **Minimum bar before commit:** `just ci` passes (fmt + clippy + all tests)
- Tests must verify **semantics**, not just "doesn't crash" (see telemetry delta tests)
- /proc parsing tests: use real data when on Linux, skip on macOS (`if !Path::new("/proc/stat").exists() { return; }`)

## Development Environment

- **Rust:** 1.88+ (install via rustup)
- **protoc:** required for proto compilation (`brew install protobuf` / `apt install protobuf-compiler`)
- **Docker:** required for `just dev-up` (TimescaleDB) and `just docker-test`
- **just:** command runner (`brew install just`)

### Local services:
```bash
just dev-up   # Starts TimescaleDB on localhost:5432 (user: pulse, pass: pulse, db: pulse)
just dev-down # Stops and removes
```

### Env for master (local dev):
```
DATABASE_URL=postgres://pulse:pulse@localhost:5432/pulse
PULSE_LISTEN=[::1]:50051
PULSE_TLS_DIR=.certs
RUST_LOG=info
```

## Security

- **Never commit** `.certs/`, `.env`, or any file containing keys/passwords
- **Never log** certificate private keys, auth tokens, or file contents from commands
- **All SQL** uses parameterized queries (`$1`, `$2`) — never string interpolation
- **Auth token** (`PULSE_REGISTER_TOKEN`) must be treated as a secret
- **Critical files:** `ca.rs` (crypto), `worker.rs` (executes arbitrary commands)
- **mTLS is mandatory** in production — reject connections without valid client cert

## Git & PR Workflow

- **Branch:** `main` (default, protected)
- **Commit format:** `type: description` — types: `feat`, `fix`, `test`, `docs`, `chore`
- **No force push** to main
- **Before push:** `just ci` must pass
- **PR expectations:** focused scope, tests for new features, docs updated if user-facing

## Important Context & File References

- `README.md` — User-facing overview + comparison table
- `docs/` — Complete wiki (install, config, API, alerting, security, troubleshooting)
- `ARCHITECTURE.md` — Technical analysis with research sources
- `migrations/001_init.sql` — TimescaleDB schema (hypertables, compression, retention)
- `grafana/dashboard.json` — Pre-built Grafana dashboard
- `dist/` — systemd service + install script
- `.github/workflows/ci.yml` — CI pipeline
- `.cargo/audit.toml` — Accepted security advisories

## Agent Behavior Guidelines

- **Read code before modifying** — especially `telemetry.rs` (kernel-specific parsing)
- **Minimal changes** — bug fix ≠ refactoring. Don't touch unrelated code.
- **Run `just ci` after every change** — never submit code that doesn't pass
- **Proto changes cascade** — see "Proto changes" section above
- **/proc parsing is tricky** — values may be cumulative counters (need delta), fields may be offset differently than expected. Always verify against kernel source.
- **Test with assertions on VALUES**, not just "doesn't crash"
- **Don't add deps without justification** — this project targets minimal binary size
