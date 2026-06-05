# Security and TLS

## Overview

All communication between agents and master is encrypted. After the first registration, connections use mutual TLS (mTLS) with automatically provisioned client certificates.

## Three Token Types

| Token | Purpose | Location |
|-------|---------|----------|
| `PULSE_REGISTER_TOKEN` | Agent registration with CA pin | `/var/lib/pulse/register-token` |
| `PULSE_ADMIN_TOKEN` | REST API write operations | `/var/lib/pulse/admin-token` |
| GitHub token | CLI binary downloads from private repo | `~/.config/pulse/config.toml` |

These are independent. Never confuse them.

## Token Format (Register Token)

```
{sha256_hex(ca.pem)}::{random_64char_hex}
```

- First half: CA fingerprint (anti-MITM pin)
- Second half: secret for authentication during Register RPC

## TLS Bootstrap Flow

```
1. Master generates CA (ca.pem + ca-key.pem) on first boot
2. Master generates register token and saves to /var/lib/pulse/register-token
3. Token is provided to agents (via install script or TUI provisioner)
4. Agent parses token → extracts ca_pin + secret
5. Agent fetches CA via GET http://master:8080/api/ca
6. Agent validates sha256(received_ca) == ca_pin → confirms server identity
7. Agent connects gRPC with TLS (trusting validated CA) → calls Register(secret)
8. Master validates secret → issues client certificate → returns cert in response
9. Agent saves certs to /etc/pulse/certs/ → updates agent.toml with [tls] section
10. Subsequent boots: agent reads [tls] section → mTLS directly (no re-registration)
```

## Security Properties

| Property | Guarantee |
|----------|-----------|
| Anti-MITM | CA pin in token — attacker cannot forge the CA |
| Agent authentication | Secret in token — only token holders can register |
| Wire encryption | TLS on all gRPC traffic |
| Post-bootstrap identity | Client cert issued and signed by master's CA |
| CA persistence | CA key saved to disk — survives restarts |
| Admin auth | Bearer token on write API endpoints |

## Important Invariants

- The server does NOT require client cert on TLS handshake (allows bootstrap)
- Authentication during Register is via token in the RPC payload, not via cert
- Client certs are issued ONLY by the Register RPC and signed by master's CA
- `CaBundle::load_or_create()` loads existing CA key (never regenerates if present)
- Token without `::` is treated as legacy/simple token (backward compatible)

## Admin Token

The `PULSE_ADMIN_TOKEN` protects destructive REST API operations:

- `DELETE /api/agents/:id` — remove agent
- `PUT /api/agents/:id/name` — rename agent
- `POST /api/agents/:id/exec` — execute commands
- `POST /api/agents/:id/file` — file operations
- `GET /api/register-token` — view register token
- `POST /api/updates/upload` — upload binaries
- `POST /api/updates/rollout` — configure rollout

Auto-generated on first boot. Stored in `/var/lib/pulse/admin-token`.

Without admin token configured: all operations are permitted (development mode).

## Systemd Hardening

Both master and agent services run with restricted privileges:

```ini
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/var/lib/pulse /etc/pulse
```

## Audit

```bash
# Check dependency vulnerabilities
cargo audit

# Check unsafe code
grep -r "unsafe" crates/ --include="*.rs"
# Result: 1 usage (statvfs syscall — necessary for disk stats, documented)
```

## What Is Never Logged

- CA private key (`ca-key.pem`)
- Client private keys
- Register token content
- Admin token content
- File contents from remote commands
