# Agent Installation (Legacy)

> **Recommended:** Use the `pulse` TUI to provision agents via SSH. See [CLI Installation](./cli-install.md).
>
> This page documents the manual `curl | sh` method for environments where the TUI is not available.

The `pulse-agent` is the monitoring daemon that runs on each server. It collects system metrics and streams them to the master over gRPC with mutual TLS.

**Binary: 3.1 MB. 2-5 MB RAM. Zero dependencies.**

## Installation

```bash
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | \
  PULSE_MASTER=https://MASTER_IP:50051 PULSE_TOKEN=token-from-master GITHUB_TOKEN=ghp_xxx sudo sh -
```

> The token is displayed in the master installation output. Format: `sha256(ca.pem)::secret`

The script:

1. Detects architecture (x86_64 / aarch64)
2. Downloads the binary from GitHub Releases
3. Creates config at `/etc/pulse/agent.toml`
4. Creates and starts the systemd service

## TLS Bootstrap (Automatic)

The agent handles the entire TLS bootstrap without manual intervention:

1. Parses token to extract `ca_pin` and `secret`
2. Fetches CA cert via `GET master:8080/api/ca`
3. Validates `sha256(ca_cert) == ca_pin` (anti-MITM)
4. Connects gRPC using the validated CA
5. Calls `Register(secret)` — receives client cert from master
6. Saves certs to `/etc/pulse/certs/` and updates `agent.toml` with `[tls]` section
7. On subsequent boots: uses mTLS directly (no re-registration)

No manual cert copying. No pre-requisites.

## Verify

```bash
systemctl status pulse-agent

# Should show "registered" or "connected"
journalctl -u pulse-agent -f

# Confirm on master
curl http://MASTER_IP:8080/api/agents
```

## Multiple Agents

Run the same command on each server. Each agent receives a unique ID during registration.

## Specific Version

```bash
curl -sfL .../install-pulse.sh | \
  VERSION=v0.4.0 PULSE_MASTER=https://MASTER_IP:50051 PULSE_TOKEN=xxx GITHUB_TOKEN=ghp_xxx sudo sh -
```

## Update

Agents self-update automatically via the master's binary server. See [Auto-Update](./auto-update.md).

For manual update, re-run the install command. The script is idempotent.

## Generated Config

The install script creates `/etc/pulse/agent.toml`:

```toml
master_endpoint = "https://MASTER_IP:50051"
metrics_interval_ms = 10000
auth_token = "sha256pin::secret"
```

After first registration, the agent adds:

```toml
[tls]
ca_cert = "/etc/pulse/certs/ca.pem"
client_cert = "/etc/pulse/certs/client.pem"
client_key = "/etc/pulse/certs/client-key.pem"
domain_name = "master"
```

## Data Directories

| Path | Contents |
|------|----------|
| `/etc/pulse/agent.toml` | Configuration |
| `/etc/pulse/certs/` | TLS certificates (auto-provisioned) |
| `/var/lib/pulse/` | Update marker, previous binary backup |
| `/usr/local/bin/pulse-agent` | Binary |
