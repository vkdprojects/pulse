# Master Installation (Legacy)

> **Recommended:** Use the `pulse` TUI to install the master via SSH. See [CLI Installation](./cli-install.md).
>
> This page documents the manual `curl | sh` method for environments where the TUI is not available.

The `pulse-master` is the central server that receives metrics, persists data, manages TLS, and serves the REST API.

**Binary: 5.8 MB. ~5 MB RAM. SQLite embedded. Zero external dependencies.**

## Installation

```bash
curl -sfL https://raw.githubusercontent.com/vkdprojects/pulse/main/dist/install-pulse.sh | \
  sudo sh -
```

The script:

1. Detects architecture (x86_64 / aarch64)
2. Downloads the binary from GitHub Releases
3. Generates TLS certificates automatically
4. Generates a register token with CA pin (format: `sha256(ca.pem)::secret`)
5. Creates and starts the systemd service
6. Prints the one-liner for installing agents

## First Boot Behavior

- TLS directory created at `/etc/pulse/` (configurable via `PULSE_TLS_DIR`)
- CA key persists in `ca-key.pem` (survives restarts, never regenerated if present)
- Register token generated and saved to `/var/lib/pulse/register-token`
- Admin token generated and saved to `/var/lib/pulse/admin-token`
- SQLite database created (zero config)
- REST API starts on `:8080`
- gRPC starts on `0.0.0.0:50051`

## Verify

```bash
systemctl status pulse-master

curl http://localhost:8080/health
# {"status":"ok"}

journalctl -u pulse-master -f
```

## Specific Version

```bash
curl -sfL .../install-pulse.sh | VERSION=v0.4.0 sudo sh -
```

## Update

Re-run the same command. The script is idempotent: stops the service, replaces the binary, restarts.

## Firewall

```bash
sudo ufw allow 50051/tcp comment "Pulse gRPC"
sudo ufw allow 8080/tcp comment "Pulse REST API"
```

## Optional: PostgreSQL Backend

By default the master uses SQLite (embedded, zero config). To scale beyond 100+ agents or integrate with Grafana, switch to PostgreSQL:

```bash
sudo systemctl edit pulse-master
```

```ini
[Service]
Environment=DATABASE_URL=postgres://pulse:password@localhost:5432/pulse
```

```bash
sudo systemctl restart pulse-master
```

The master detects `postgres://` in the URL and activates hypertables, compression, continuous aggregates, and retention policies (requires TimescaleDB extension).

## Data Directories

| Path | Contents |
|------|----------|
| `/etc/pulse/` | TLS certificates (CA, server cert/key) |
| `/var/lib/pulse/` | SQLite database, register-token, admin-token, update binaries |
| `/usr/local/bin/pulse-master` | Binary |
