# Docker (Optional)

> Docker is NOT required to use Pulse. The standard installation uses a single binary with SQLite embedded.
>
> Docker Compose is useful for: quick demos, local development with Grafana, or environments that want TimescaleDB.

## Demo Mode (Docker Compose + Grafana)

```bash
docker compose up --build
```

This starts: TimescaleDB + Master + 3 Agents + Grafana.

| Service | Port | Description |
|---------|------|-------------|
| `db` | 5432 | TimescaleDB (PostgreSQL 17) |
| `master` | 50051 | pulse-master with TLS |
| `agent-1/2/3` | — | 3 simulated Linux agents |
| `grafana` | 3000 | Pre-configured dashboard (admin/pulse) |

## E2E Automated Test

```bash
./tests/e2e-docker.sh
```

Validates 7 checks:

1. 3 agents registered in DB
2. Metrics persisted
3. CPU data present
4. Memory data present
5. TLS functional
6. Agents unique
7. Hypertables active

## Local Development

```bash
# Start only the database (TimescaleDB)
just dev-up

# Run master locally (terminal 1)
just dev-master

# Run agent locally (terminal 2)
just dev-agent

# Stop everything
just dev-down
```

## Useful DB Queries (TimescaleDB)

```sql
-- Registered agents
SELECT id, hostname, os, arch, status, last_seen_at FROM agents;

-- CPU last 10 readings
SELECT time, agent_id, cpu_total_bp/100.0 as cpu_pct
FROM metrics ORDER BY time DESC LIMIT 10;

-- Memory per agent
SELECT time, agent_id, mem_used/1048576 as used_mb, mem_total/1048576 as total_mb
FROM metrics WHERE agent_id = 'agent-0001' ORDER BY time DESC LIMIT 5;

-- Network delta
SELECT time, agent_id, interface, rx_bytes, tx_bytes
FROM net_metrics ORDER BY time DESC LIMIT 10;

-- Containers
SELECT time, agent_id, container_name, mem_usage/1048576 as mem_mb
FROM container_metrics ORDER BY time DESC LIMIT 10;
```

## Rebuild a Single Service

```bash
docker compose build master
docker compose up -d master
```
