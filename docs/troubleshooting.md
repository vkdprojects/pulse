# Troubleshooting

## Agent Does Not Connect

**Symptom:** `connection lost, reconnecting` in agent logs.

| Cause | Solution |
|-------|----------|
| Master offline | `systemctl status pulse-master` |
| Firewall blocking | Open port 50051/tcp (gRPC) and 8080/tcp (REST) |
| Wrong token | Verify token was copied correctly from master install output |
| CA fetch failed | Verify `http://master:8080/api/ca` responds |
| Wrong master IP/DNS | Check `master_endpoint` in `/etc/pulse/agent.toml` |

**Debug:**

```bash
journalctl -u pulse-agent -f

# Test connectivity
curl http://MASTER_IP:8080/health
curl http://MASTER_IP:8080/api/ca
```

---

## Agent Registered But No Metrics

Check master logs:

```bash
journalctl -u pulse-master | grep "metrics"
```

If the agent appears in `GET /api/agents` but without metrics: likely intermittent network issue. The agent reconnects automatically with exponential backoff (1s to 60s).

---

## CPU Shows 0%

The first reading after boot/restart shows 0% (no baseline for delta calculation). This is normal — the second reading onward shows real values.

---

## Network Shows 0 Bytes

Same reason — first reading after restart is 0 (delta requires 2 samples). Normal.

---

## Agent Uses More Memory Than Expected

Normal range: 2-5 MB. If growing beyond that:

```bash
systemctl restart pulse-agent
```

---

## Master Does Not Start

| Error | Solution |
|-------|----------|
| `CA init failed` | Check write permissions on `PULSE_TLS_DIR` (default: `/etc/pulse/`) |
| `bind failed` | Port 50051 or 8080 already in use |
| `database error` (with DATABASE_URL) | PostgreSQL not running or wrong credentials |

```bash
journalctl -u pulse-master --no-pager -n 50
```

---

## Health Check Fails

```bash
curl http://localhost:8080/health
# Expected: {"status":"ok"}
```

If no response: master is not listening on 8080. Check logs for bind errors.

---

## TUI Cannot Connect to Master

| Cause | Solution |
|-------|----------|
| Wrong URL | Check `url` in `~/.config/pulse/config.toml` |
| Network | Verify `curl http://master:8080/health` works from your machine |
| Admin token wrong | Operations will fail with 401 — update token in config |

---

## Auto-Update Not Working

1. Check rollout is configured: `curl http://master:8080/api/updates/rollout`
2. Check agent version is different from target
3. Agents check every 5 minutes — wait at least one cycle
4. Check if agent is within rollout percentage (`hash(agent_id) % 100 < rollout_pct`)
5. Agents pre-v0.4.0 do not have updater — require one manual reinstall

---

## Reinstall / Full Reset

```bash
# Uninstall
systemctl disable --now pulse-master  # or pulse-agent
rm /usr/local/bin/pulse-master        # or pulse-agent
rm -rf /etc/pulse
rm -rf /var/lib/pulse

# Reinstall
curl -sfL .../install-pulse.sh | GITHUB_TOKEN=ghp_xxx sudo sh -
```

---

## Disk Full (TimescaleDB)

```sql
-- Check hypertable sizes
SELECT hypertable_name, pg_size_pretty(hypertable_size(format('%I', hypertable_name)::regclass))
FROM timescaledb_information.hypertables;

-- Force compression
SELECT compress_chunk(c) FROM show_chunks('metrics', older_than => interval '1 day') c;

-- Force retention
SELECT drop_chunks('metrics', older_than => interval '30 days');
```

---

## Old Data Accumulating (SQLite)

The retention worker automatically deletes data older than `PULSE_RETENTION_DAYS` (default: 30). To adjust:

```bash
sudo systemctl edit pulse-master
```

```ini
[Service]
Environment=PULSE_RETENTION_DAYS=7
```

```bash
sudo systemctl restart pulse-master
```
