# Configuration

## Master (Environment Variables)

The master is configured exclusively via environment variables. All defaults work out of the box — configure only what you need to change.

### Core

| Variable | Default | Description |
|----------|---------|-------------|
| `PULSE_LISTEN` | `[::]:50051` | gRPC listen address |
| `PULSE_TLS_DIR` | `/etc/pulse` | Directory for TLS certificates (auto-created) |
| `DATABASE_URL` | _(SQLite)_ | If `postgres://...`, uses PostgreSQL/TimescaleDB |
| `PULSE_REGISTER_TOKEN` | _(auto-generated)_ | Token for agent registration (format: `ca_pin::secret`) |
| `PULSE_ADMIN_TOKEN` | _(auto-generated)_ | Token for admin REST API operations |
| `PULSE_RETENTION_DAYS` | `30` | Days to keep metrics before deletion |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |

### Alerts

| Variable | Default | Description |
|----------|---------|-------------|
| `PULSE_ALERT_WEBHOOK` | _(disabled)_ | Webhook URL (Telegram/Slack/Discord/generic) |
| `PULSE_ALERT_TELEGRAM_CHAT_ID` | — | Chat ID for Telegram webhooks |
| `PULSE_ALERT_CPU_PCT` | `90` | CPU threshold (%) |
| `PULSE_ALERT_MEM_PCT` | `90` | Memory threshold (%) |
| `PULSE_ALERT_BW_MBPS` | `100` | Bandwidth threshold (MB/s) |

### Modifying Configuration

```bash
sudo systemctl edit pulse-master
```

```ini
[Service]
Environment=PULSE_RETENTION_DAYS=60
Environment=PULSE_ALERT_WEBHOOK=https://api.telegram.org/bot<TOKEN>/sendMessage
Environment=PULSE_ALERT_TELEGRAM_CHAT_ID=123456
```

```bash
sudo systemctl restart pulse-master
```

### Data Paths

| Path | Contents |
|------|----------|
| `/etc/pulse/` | CA cert, CA key, server cert, server key |
| `/var/lib/pulse/pulse.db` | SQLite database (default backend) |
| `/var/lib/pulse/register-token` | Register token for agents |
| `/var/lib/pulse/admin-token` | Admin token for REST API |
| `/var/lib/pulse/updates/` | Uploaded binaries for auto-update |

---

## Agent (`/etc/pulse/agent.toml`)

Generated automatically by the install script or TUI provisioner. Edit only if manual adjustment is needed.

```toml
master_endpoint = "https://master.example.com:50051"
metrics_interval_ms = 10000
auth_token = "sha256pin::secret"
labels = ["prod", "datacenter-sp"]
```

After first registration, the agent appends automatically:

```toml
[tls]
ca_cert = "/etc/pulse/certs/ca.pem"
client_cert = "/etc/pulse/certs/client.pem"
client_key = "/etc/pulse/certs/client-key.pem"
domain_name = "master"
```

### Fields

| Field | Default | Description |
|-------|---------|-------------|
| `master_endpoint` | `http://[::1]:50051` | Master gRPC URL |
| `metrics_interval_ms` | `10000` | Collection interval in milliseconds |
| `auth_token` | `""` | Token with CA pin for bootstrap |
| `labels` | `[]` | Tags for grouping/filtering agents |
| `tls.ca_cert` | — | CA certificate path (auto-populated) |
| `tls.client_cert` | — | Client certificate path (auto-populated) |
| `tls.client_key` | — | Client private key path (auto-populated) |
| `tls.domain_name` | `localhost` | Domain for TLS validation |

---

## CLI (`~/.config/pulse/config.toml`)

Created by the first-run setup wizard. Supports multiple masters.

```toml
github_token = "ghp_..."
github_repo = "gcnsolucoes/vixpi-pulse"
current_context = "production"

[[masters]]
name = "production"
url = "http://master-prod:8080"
admin_token = "admin-token-here"

[[masters]]
name = "homelab"
url = "http://192.168.68.66:8080"
admin_token = "another-token"
```

### Fields

| Field | Description |
|-------|-------------|
| `github_token` | GitHub PAT for downloading binaries (repo scope) |
| `github_repo` | Repository in `owner/repo` format |
| `current_context` | Name of the active master |
| `masters[].name` | Display name for the master |
| `masters[].url` | REST API URL (http://host:8080) |
| `masters[].admin_token` | Admin token for write operations |

---

## Version

Binary version comes from git tags via `build.rs` using `git describe --tags`. Never edit `Cargo.toml` version for releases — tag the commit instead.
