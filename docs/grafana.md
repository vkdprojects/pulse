# Grafana Dashboard

> Grafana is optional. Requires PostgreSQL/TimescaleDB as backend — does not work with SQLite.

## Via Docker Compose (Easiest)

```bash
docker compose up --build
```

Grafana available at **http://localhost:3000**

- User: `admin`
- Password: `pulse`
- Dashboard pre-loaded: "Vixpi Pulse - Infrastructure"

## Manual Setup

1. Configure master with TimescaleDB:

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

2. Add PostgreSQL datasource in Grafana:
   - URL: `localhost:5432`
   - Database: `pulse`
   - User: `pulse`
   - TimescaleDB: enabled

3. Import dashboard:
   - Grafana > Dashboards > Import
   - Upload `grafana/dashboard.json`

## Included Panels

| Panel | Query |
|-------|-------|
| CPU Usage (%) | `cpu_total_bp / 100` per agent |
| Memory Used (MB) | `mem_used / 1048576` per agent |
| Load Average (1m) | `load_1m / 100` |
| Network Traffic | `rx_bytes` + `tx_bytes` per interface |
| Container Memory | `mem_usage` per container |
| Agents Inventory | Table with id, hostname, os, arch, last_seen |

## Custom Queries

```sql
-- Average CPU per hour (continuous aggregate)
SELECT bucket, agent_id, avg_cpu_bp/100.0 as cpu_pct
FROM metrics_hourly
WHERE bucket > now() - interval '24 hours';

-- Top agents by memory
SELECT agent_id, max(mem_used)/1048576 as max_mem_mb
FROM metrics
WHERE time > now() - interval '1 hour'
GROUP BY agent_id ORDER BY max_mem_mb DESC;

-- Offline agents
SELECT id, hostname, last_seen_at, now() - last_seen_at as downtime
FROM agents WHERE status = 'offline';
```
