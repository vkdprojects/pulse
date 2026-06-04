# REST API

O master expõe uma REST API na porta **8080**.

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

### CA Certificate

```
GET /api/ca
```

Retorna o certificado CA em formato PEM (`text/plain`). Usado pelo agent no TLS bootstrap.

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

### Sessions / Info de um Agent

```
GET /api/agents/:id/sessions
```

```json
{
  "agent_id": "agent-0001",
  "hostname": "server-prod-1",
  "status": "online",
  "last_seen": "2026-06-03 21:00:00+00"
}
```

---

## Exemplos com curl

```bash
# Health
curl http://localhost:8080/health

# CA cert
curl http://localhost:8080/api/ca

# Listar agents
curl http://localhost:8080/api/agents

# Métricas
curl http://localhost:8080/api/agents/agent-0001/metrics

# Info do agent
curl http://localhost:8080/api/agents/agent-0001/sessions
```

## Uso com jq

```bash
# Agents online
curl -s http://localhost:8080/api/agents | jq '.agents[] | select(.status == "online")'

# CPU% atual
curl -s http://localhost:8080/api/agents/agent-0001/metrics | jq '.metrics[0].cpu_pct'
```
