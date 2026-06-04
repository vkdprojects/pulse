# Grafana Dashboard

## Acesso rápido (Docker)

Com `docker compose up`, Grafana está em **http://localhost:3000**

- User: `admin`
- Password: `pulse`
- Dashboard pré-carregado: "Vixpi Pulse - Infrastructure"

## Setup manual

1. Adicionar datasource PostgreSQL:
   - URL: `db:5432` (ou IP do TimescaleDB)
   - Database: `pulse`
   - User: `pulse`
   - TLS: off (ou configure)
   - TimescaleDB: **habilitado**

2. Importar dashboard:
   - Grafana → Dashboards → Import
   - Upload `grafana/dashboard.json`

## Painéis incluídos

| Painel | Query |
|---|---|
| CPU Usage (%) | `cpu_total_bp / 100` por agent |
| Memory Used (MB) | `mem_used / 1048576` por agent |
| Load Average (1m) | `load_1m / 100` |
| Network Traffic | `rx_bytes` + `tx_bytes` por interface |
| Container Memory | `mem_usage` por container |
| Agents Inventory | Tabela com id, hostname, os, arch, last_seen |

## Queries customizadas úteis

```sql
-- CPU médio por hora (usa continuous aggregate)
SELECT bucket, agent_id, avg_cpu_bp/100.0 as cpu_pct
FROM metrics_hourly
WHERE bucket > now() - interval '24 hours';

-- Top agents por memória
SELECT agent_id, max(mem_used)/1048576 as max_mem_mb
FROM metrics
WHERE time > now() - interval '1 hour'
GROUP BY agent_id ORDER BY max_mem_mb DESC;

-- Agents offline
SELECT id, hostname, last_seen_at, now() - last_seen_at as downtime
FROM agents WHERE status = 'offline';
```
