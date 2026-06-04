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

```
-----BEGIN CERTIFICATE-----
MIIBqTCCAV+gAwIBAgIUZ...
-----END CERTIFICATE-----
```

---

### Overview (Dashboard)

```
GET /api/overview
```

Resumo de todos os agents com métricas atuais.

```json
{
  "total_agents": 3,
  "online": 2,
  "offline": 1,
  "agents": [
    {
      "id": "agent-0001",
      "hostname": "server-us-east-1",
      "status": "online",
      "cpu_pct": 23.5,
      "mem_pct": 62.1,
      "uptime_secs": 86400
    },
    {
      "id": "agent-0002",
      "hostname": "server-eu-west-1",
      "status": "online",
      "cpu_pct": 45.2,
      "mem_pct": 78.3,
      "uptime_secs": 172800
    },
    {
      "id": "agent-0003",
      "hostname": "server-br-south-1",
      "status": "offline",
      "cpu_pct": 0.0,
      "mem_pct": 0.0,
      "uptime_secs": 0
    }
  ]
}
```

---

### Alertas

```
GET /api/alerts
```

Histórico de alertas disparados.

```json
{
  "alerts": [
    {
      "time": "2026-06-03 21:15:00+00",
      "agent_id": "agent-0001",
      "alert_type": "cpu_high",
      "message": "CPU > 90% for 3 consecutive readings (current: 94.2%)"
    },
    {
      "time": "2026-06-03 20:30:00+00",
      "agent_id": "agent-0003",
      "alert_type": "agent_offline",
      "message": "Agent offline for > 45 seconds"
    }
  ]
}
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
      "version": "0.2.0",
      "status": "online"
    },
    {
      "id": "agent-0002",
      "hostname": "server-eu-west-1",
      "os": "linux",
      "arch": "aarch64",
      "version": "0.2.0",
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
    },
    {
      "time": "2026-06-03 20:59:50+00",
      "cpu_pct": 21.8,
      "mem_used_mb": 1020,
      "mem_total_mb": 4096,
      "uptime_secs": 86390
    }
  ]
}
```

---

### Última Métrica (Snapshot)

```
GET /api/agents/:id/metrics/latest
```

Retorna apenas a leitura mais recente com campos extras.

```json
{
  "agent_id": "agent-0001",
  "time": "2026-06-03 21:00:00+00",
  "cpu_pct": 23.5,
  "mem_pct": 62.1,
  "mem_used_mb": 1024,
  "mem_total_mb": 4096,
  "uptime_secs": 86400,
  "battery_pct": 85
}
```

---

### Discos

```
GET /api/agents/:id/disks
```

Uso de disco por mountpoint.

```json
{
  "agent_id": "agent-0001",
  "disks": [
    {
      "mount_point": "/",
      "total_gb": 100.0,
      "used_gb": 45.2,
      "used_pct": 45.2
    },
    {
      "mount_point": "/data",
      "total_gb": 500.0,
      "used_gb": 312.7,
      "used_pct": 62.5
    }
  ]
}
```

---

### Rede

```
GET /api/agents/:id/network
```

Tráfego por interface (delta desde última leitura).

```json
{
  "agent_id": "agent-0001",
  "interfaces": [
    {
      "name": "eth0",
      "rx_bytes": 1048576,
      "tx_bytes": 524288
    },
    {
      "name": "lo",
      "rx_bytes": 4096,
      "tx_bytes": 4096
    }
  ]
}
```

---

### Containers

```
GET /api/agents/:id/containers
```

Docker/Podman containers detectados via cgroup.

```json
{
  "agent_id": "agent-0001",
  "containers": [
    {
      "id": "a1b2c3d4e5f6",
      "name": "nginx-proxy",
      "image": "nginx:alpine",
      "state": "running",
      "cpu_pct": 2.1,
      "mem_mb": 32.5
    },
    {
      "id": "f6e5d4c3b2a1",
      "name": "postgres-db",
      "image": "postgres:17",
      "state": "running",
      "cpu_pct": 8.3,
      "mem_mb": 256.0
    }
  ]
}
```

---

### Processos

```
GET /api/agents/:id/processes
```

Top processos por CPU + memória.

```json
{
  "agent_id": "agent-0001",
  "processes": [
    {
      "pid": 1234,
      "name": "postgres",
      "cpu_pct": 12.5,
      "rss_mb": 256.0
    },
    {
      "pid": 5678,
      "name": "nginx",
      "cpu_pct": 3.2,
      "rss_mb": 48.0
    }
  ]
}
```

---

### GPU

```
GET /api/agents/:id/gpu
```

Stats de GPUs NVIDIA (requer `nvidia-smi` no agent).

```json
{
  "agent_id": "agent-0001",
  "gpus": [
    {
      "index": 0,
      "name": "NVIDIA RTX 4090",
      "utilization_pct": 78.5,
      "mem_used_mb": 8192,
      "mem_total_mb": 24576,
      "temp_c": 72,
      "power_watts": 320.5
    }
  ]
}
```

---

### Conexões TCP (Netflow)

```
GET /api/agents/:id/connections
```

Conexões TCP agrupadas por porta local com IPs remotos.

```json
{
  "agent_id": "agent-0001",
  "connections": [
    {
      "port": 22,
      "service": "ssh",
      "active_count": 3,
      "remote_ips": ["203.0.113.10", "198.51.100.5", "192.0.2.1"]
    },
    {
      "port": 443,
      "service": "https",
      "active_count": 47,
      "remote_ips": ["203.0.113.10", "198.51.100.5", "10.0.0.1"]
    },
    {
      "port": 5432,
      "service": "postgres",
      "active_count": 5,
      "remote_ips": ["10.0.0.2", "10.0.0.3"]
    }
  ]
}
```

---

### Sessions (Info do Agent)

```
GET /api/agents/:id/sessions
```

Informações e status do agent.

```json
{
  "agent_id": "agent-0001",
  "hostname": "server-prod-1",
  "status": "online",
  "last_seen": "2026-06-03 21:00:00+00"
}
```

---

### Remover Agent

```
DELETE /api/agents/:id
```

Remove um agent e seus dados.

```json
{
  "deleted": true,
  "agent_id": "agent-0001"
}
```

---

### Executar Comando Remoto

```
POST /api/agents/:id/exec
```

> ⚠️ **Em breve** — atualmente retorna 501.

```json
{
  "error": "Remote exec via REST coming soon..."
}
```

---

## Exemplos com curl

```bash
# Health check
curl http://localhost:8080/health

# CA certificate (salvar em arquivo)
curl http://localhost:8080/api/ca -o ca.pem

# Overview de todos os servidores
curl -s http://localhost:8080/api/overview

# Listar agents
curl -s http://localhost:8080/api/agents

# Alertas recentes
curl -s http://localhost:8080/api/alerts

# Métricas (últimas 20 leituras)
curl -s http://localhost:8080/api/agents/agent-0001/metrics

# Snapshot atual
curl -s http://localhost:8080/api/agents/agent-0001/metrics/latest

# Discos
curl -s http://localhost:8080/api/agents/agent-0001/disks

# Rede
curl -s http://localhost:8080/api/agents/agent-0001/network

# Containers
curl -s http://localhost:8080/api/agents/agent-0001/containers

# Processos
curl -s http://localhost:8080/api/agents/agent-0001/processes

# GPU
curl -s http://localhost:8080/api/agents/agent-0001/gpu

# Conexões TCP
curl -s http://localhost:8080/api/agents/agent-0001/connections

# Info do agent
curl -s http://localhost:8080/api/agents/agent-0001/sessions

# Remover agent
curl -X DELETE http://localhost:8080/api/agents/agent-0001
```

---

## Uso com jq

```bash
# Agents online
curl -s http://localhost:8080/api/agents | jq '.agents[] | select(.status == "online")'

# CPU% atual de todos os agents
curl -s http://localhost:8080/api/overview | jq '.agents[] | {hostname, cpu_pct, mem_pct}'

# Quem está acessando porta 22 (SSH)?
curl -s http://localhost:8080/api/agents/agent-0001/connections | jq '.connections[] | select(.service=="ssh")'

# Discos com mais de 80% de uso
curl -s http://localhost:8080/api/agents/agent-0001/disks | jq '.disks[] | select(.used_pct > 80)'

# Containers rodando
curl -s http://localhost:8080/api/agents/agent-0001/containers | jq '.containers[] | select(.state=="running") | {name, cpu_pct, mem_mb}'

# Top processo por CPU
curl -s http://localhost:8080/api/agents/agent-0001/processes | jq '.processes | sort_by(-.cpu_pct) | .[0]'

# Todas as portas com conexões externas
curl -s http://localhost:8080/api/agents/agent-0001/connections | jq '.connections[] | {port, service, active_count}'
```
