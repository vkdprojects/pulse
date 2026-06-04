# REST API

O master expõe uma REST API na porta **8080** para consulta de dados e operações.

Base URL: `http://MASTER_IP:8080`

---

## Endpoints

### Health Check

```
GET /health
```

```json
{"status": "ok"}
```

---

### Listar Agents

```
GET /api/agents
```

Retorna todos os agents registrados com status.

```json
{
  "agents": [
    {
      "id": "agent-0001",
      "hostname": "server-us-east-1",
      "os": "linux",
      "arch": "x86_64",
      "version": "0.1.0",
      "status": "online"
    },
    {
      "id": "agent-0002",
      "hostname": "server-eu-west-1",
      "os": "linux",
      "arch": "aarch64",
      "version": "0.1.0",
      "status": "offline"
    }
  ]
}
```

---

### Métricas de um Agent

```
GET /api/agents/:id/metrics
```

Retorna as últimas 20 leituras do agent.

```json
{
  "agent_id": "agent-0001",
  "metrics": [
    {
      "time": "2026-06-03 21:00:00+00",
      "cpu_pct": 23.5,
      "mem_used_mb": 1024,
      "mem_total_mb": 4096,
      "uptime_secs": 86400
    }
  ]
}
```

---

## Exemplos com curl

```bash
# Health
curl http://localhost:8080/health

# Listar agents
curl http://localhost:8080/api/agents

# Métricas do agent-0001
curl http://localhost:8080/api/agents/agent-0001/metrics
```

---

## Uso com jq (formatar output)

```bash
# Agents online
curl -s http://localhost:8080/api/agents | jq '.agents[] | select(.status == "online")'

# CPU% atual de cada agent
curl -s http://localhost:8080/api/agents/agent-0001/metrics | jq '.metrics[0].cpu_pct'
```
