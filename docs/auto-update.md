# Auto-Update System

Agents self-update without SSH access or manual intervention. Push a git tag, and agents update within 5 minutes.

## How It Works

```
git tag v0.5.0 && git push --tags
  → CI builds binaries (musl x86_64 + aarch64)
  → CI uploads to master (POST /api/updates/upload)
  → CI sets rollout target (POST /api/updates/rollout)
  → Agents poll every 5 minutes (GET /api/updates/check)
  → Download, SHA256 verify, atomic replace, restart
```

## Lifecycle

1. CI compiles static binaries and uploads them to the master via REST API
2. CI sets the target version and rollout percentage
3. Agents check for updates every 5 minutes
4. If an update is available and the agent is within the rollout percentage:
   - Download binary from master (`GET /api/updates/download`)
   - Verify SHA256 checksum
   - Save current binary as `.prev` (for rollback)
   - Atomic replace of the binary
   - Write update marker to `/var/lib/pulse/update-marker`
   - Exit with code 0 (systemd restarts with new binary)

## Gradual Rollout

Rollout is deterministic: `hash(agent_id) % 100 < rollout_pct`. The same agent is always in the same bucket.

```bash
# Canary: 10% of agents first
curl -X POST http://master:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"version":"v0.5.0","rollout_pct":10}'

# Expand to all
curl -X POST http://master:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"rollout_pct":100}'

# Rollback: point to previous version
curl -X POST http://master:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"version":"v0.4.0","rollout_pct":100}'
```

## Urgent Update

```bash
curl -X POST http://master:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"version":"v0.5.0","rollout_pct":100,"urgent":true}'
```

With `urgent: true`, agents apply immediately (no jitter delay of 0-300s).

## Automatic Rollback

If the agent crashes 3 consecutive times after an update:

1. Detects repeated crashes via marker file (`/var/lib/pulse/update-marker`)
2. Restores previous binary from `.prev`
3. Restarts with the stable version

## Security

- Binaries served by the master (agents do not need GitHub access)
- Downloads authenticated (agents are already registered via mTLS)
- SHA256 verified before applying
- Path sanitized (no directory traversal)
- Upload and rollout endpoints require `PULSE_ADMIN_TOKEN`

## API Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/api/updates/check` | GET | None | Agent checks if update available |
| `/api/updates/download` | GET | None | Agent downloads binary |
| `/api/updates/upload` | POST | Admin | CI uploads binary (max 50 MB) |
| `/api/updates/rollout` | GET | None | View current rollout config |
| `/api/updates/rollout` | POST | Admin | Set target version and rollout |

## CI Integration

The release workflow (`.github/workflows/ci.yml`) does this automatically on tag push:

1. Build static musl binaries (x86_64 + aarch64) and macOS (arm64 + x86_64)
2. Create GitHub Release with all binaries
3. Upload linux binaries to master via `POST /api/updates/upload`
4. Set target version + rollout via `POST /api/updates/rollout`

Required GitHub configuration:

- **Variable:** `PULSE_MASTER_URL` = `http://your-master:8080`
- **Secret:** `PULSE_ADMIN_TOKEN` = admin token from master

## Verify Status

```bash
# View rollout config
curl http://master:8080/api/updates/rollout

# Version of each agent
curl -s http://master:8080/api/overview | jq '.agents[] | {hostname, version}'

# Via TUI: press 'u' on Home or Fleet screen
```

## Agents Without the Updater

Agents installed before the auto-update feature (pre-v0.4.0) do not have the updater logic. They require one manual reinstall to get the new binary with updater support. After that, all updates are automatic.
