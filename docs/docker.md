# Docker & Grafana (opcional)

> **Docker NÃO é requisito para usar o Vixpi Pulse.** A instalação padrão é via `curl | sh` com SQLite embedded.
>
> Docker Compose é útil para: demo rápida, desenvolvimento local com Grafana, ou ambiente com TimescaleDB.

## Demo Mode (Docker Compose + Grafana)

```bash
docker compose up --build
```

Sobe: TimescaleDB + Master + 3 Agents + Grafana

| Serviço | Porta | Descrição |
|---|---|---|
| `db` | 5432 | TimescaleDB (PostgreSQL 17) |
| `master` | 50051 | pulse-master com TLS |
| `agent-1/2/3` | — | 3 agents Linux simulados |
| `grafana` | 3000 | Dashboard pré-configurado (admin/pulse) |

## Teste E2E automatizado

```bash
./tests/e2e-docker.sh
```

Valida 7 checks:
1. 3 agents registrados no DB
2. Métricas persistidas
3. CPU data presente
4. Memory data presente
5. TLS funcional
6. Agents únicos
7. Hypertables ativas

## Dev local

```bash
# 1. Sobe apenas o banco (para usar TimescaleDB localmente)
just dev-up

# 2. Roda master local (terminal 1)
just dev-master

# 3. Roda agent local (terminal 2)
just dev-agent

# 4. Para tudo
just dev-down
```

## Consultas úteis no DB (quando usando TimescaleDB)

```sql
-- Agents registrados
SELECT id, hostname, os, arch, status, last_seen_at FROM agents;

-- CPU últimas 10 leituras
SELECT time, agent_id, cpu_total_bp/100.0 as cpu_pct
FROM metrics ORDER BY time DESC LIMIT 10;

-- Memória por agent
SELECT time, agent_id, mem_used/1048576 as used_mb, mem_total/1048576 as total_mb
FROM metrics WHERE agent_id = 'agent-0001' ORDER BY time DESC LIMIT 5;

-- Network delta
SELECT time, agent_id, interface, rx_bytes, tx_bytes
FROM net_metrics ORDER BY time DESC LIMIT 10;

-- Containers
SELECT time, agent_id, container_name, mem_usage/1048576 as mem_mb
FROM container_metrics ORDER BY time DESC LIMIT 10;
```

## Rebuild apenas um serviço

```bash
docker compose build master
docker compose up -d master
```
