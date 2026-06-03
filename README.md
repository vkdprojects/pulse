# Vixpi Pulse

Ultra-lightweight infrastructure monitoring agent. Streams hardware telemetry over gRPC with mutual TLS.

**Target: 2-5 MB RAM idle** on production Linux hosts.

## Architecture

```
┌─────────────┐    gRPC/mTLS     ┌──────────────┐
│ pulse-agent │ ──────────────▶  │ pulse-master │
│  (per host) │ ◀──────────────  │  (central)   │
└─────────────┘   commands       └──────────────┘
```

- **pulse-agent**: Collects CPU, memory, disk, network from `/proc` + `/sys/fs/cgroup`. Executes remote commands.
- **pulse-master**: Receives metrics streams, dispatches commands to agents.
- **pulse-proto**: Protobuf/gRPC contract shared between both.

## Quick Start

```bash
# Prerequisites: Rust 1.88+, protoc
brew install protobuf  # macOS
# apt install protobuf-compiler  # Linux

# Build
cargo build --workspace

# Generate dev certificates (CA + server + client)
just gen-certs

# Run master (terminal 1)
just run-master-tls

# Run agent (terminal 2)
just run-agent-tls
```

## Configuration

### Agent (`pulse-agent.toml`)

```toml
master_endpoint = "https://master.example.com:50051"
metrics_interval_ms = 10000
labels = ["prod", "us-east-1"]

[tls]
ca_cert = "/etc/pulse/ca.pem"
client_cert = "/etc/pulse/agent.pem"
client_key = "/etc/pulse/agent-key.pem"
```

Pass config path as first argument: `pulse-agent /etc/pulse-agent.toml`

Without a config file, the agent connects to `http://[::1]:50051` insecurely.

### Master (environment variables)

| Variable | Default | Description |
|---|---|---|
| `PULSE_LISTEN` | `[::1]:50051` | Listen address |
| `PULSE_TLS_DIR` | _(none)_ | Directory with `ca.pem`, `server.pem`, `server-key.pem` |
| `RUST_LOG` | `info` | Log level filter |

### Certificate Generation

```bash
# Generates .certs/{ca,server,server-key,client,client-key}.pem
cargo run -p pulse-master -- gen-certs .certs
```

For production, use your own CA. The built-in CA is for development/testing only.

## Telemetry Collected

| Metric | Source | Notes |
|---|---|---|
| CPU usage (per-core + total) | `/proc/stat` | Basis points (0-10000) |
| Memory (total, used, available, swap) | `/proc/meminfo` | |
| Memory (cgroup limit/usage) | `/sys/fs/cgroup/memory.*` | v1 + v2 auto-detection |
| Network (rx/tx bytes, packets, errors) | `/proc/net/dev` | Per interface |
| System (uptime, processes) | `/proc/uptime`, `/proc/stat` | |
| Disk mounts | `/proc/mounts` | ext4, xfs, btrfs, zfs, f2fs |

## Remote Command Execution

The master can send commands to agents via the bidirectional `CommandChannel`:

- **Exec**: Run arbitrary programs with args, env, working dir, timeout
- **Package**: Install/remove/update packages (apt-get)
- **File**: Read/write/append files

stdout/stderr are streamed in real-time back to the master.

## Development

```bash
just --list        # Show all commands
just ci            # Run full CI (fmt + clippy + test)
just test          # Run tests only
just lint          # fmt-check + clippy
just dev           # Generate certs + show run instructions
```

## Testing

```bash
cargo test --workspace
```

Integration tests cover:
- gRPC register (plaintext)
- Metrics streaming (plaintext)
- mTLS successful connection
- mTLS rejection: no client certificate
- mTLS rejection: wrong CA

## Release Build

```bash
just release-agent
# Binary at: target/release/pulse-agent
# Optimized: opt-level=z, LTO, codegen-units=1, strip
```

For cross-compilation to Linux musl:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release -p pulse-agent --target x86_64-unknown-linux-musl
```

## Project Structure

```
crates/
├── pulse-proto/     # Protobuf definitions + generated code
│   ├── proto/pulse.proto
│   └── src/lib.rs
├── pulse-agent/     # The monitoring daemon
│   └── src/
│       ├── main.rs       # Entry point, loop, reconnect
│       ├── config.rs     # TOML config + defaults
│       ├── telemetry.rs  # /proc parsing (CPU, mem, net, disk)
│       └── worker.rs     # Command execution + output streaming
├── pulse-master/    # The central server
│   └── src/
│       ├── main.rs       # Entry point, TLS setup
│       ├── lib.rs        # Public API + cert generation
│       ├── service.rs    # gRPC service implementation
│       └── ca.rs         # X.509 CA (rcgen)
└── pulse-tests/     # Integration tests
    └── tests/integration.rs
```

## License

MIT
