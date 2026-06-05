# REST API Reference

The master exposes a REST API on port **8080**.

Base URL: `http://MASTER_IP:8080`

## Authentication

Endpoints are divided into two groups:

- **Public** — Read-only access (overview, metrics, health)
- **Admin** — Write operations (delete, exec, upload, rollout, rename)

Admin endpoints require the `Authorization: Bearer <PULSE_ADMIN_TOKEN>` header.

---

## Public Endpoints

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

Returns the CA certificate in PEM format (`application/x-pem-file`). Used by agents during TLS bootstrap.

---

### Overview

```
GET /api/overview
```

All agents with their latest metrics.

```json
{
  "total_agents": 3,
  "online": 2,
  "offline": 1,
  "agents": [
    {
      "id": "agent-0001",
      "hostname": "web-prod-01",
      "status": "online",
      "cpu_pct": 23.5,
      "mem_pct": 62.1,
      "uptime_secs": 86400,
      "version": "0.4.0"
    }
  ]
}
```

---

### List Agents

```
GET /api/agents
```

```json
{
  "agents": [
    {
      "id": "agent-0001",
      "hostname": "web-prod-01",
      "os": "linux",
      "arch": "x86_64",
      "version": "0.4.0",
      "status": "online"
    }
  ]
}
```

---

### Metrics History

```
GET /api/agents/:id/metrics
```

Last 20 readings for the agent.

```json
{
  "agent_id": "agent-0001",
  "metrics": [
    {
      "time": "2026-06-03T21:00:00+00:00",
      "cpu_pct": 23.5,
      "mem_used_mb": 1024,
      "mem_total_mb": 4096,
      "uptime_secs": 86400
    }
  ]
}
```

---

### Latest Metrics (Snapshot)

```
GET /api/agents/:id/metrics/latest
```

Most recent reading with extended fields.

```json
{
  "agent_id": "agent-0001",
  "time": "2026-06-03T21:00:00+00:00",
  "cpu_pct": 23.5,
  "mem_pct": 62.1,
  "mem_used_mb": 1024,
  "mem_total_mb": 4096,
  "uptime_secs": 86400,
  "battery_pct": 85
}
```

---

### Disks

```
GET /api/agents/:id/disks
```

Disk usage per mount point.

```json
{
  "agent_id": "agent-0001",
  "disks": [
    {
      "mount_point": "/",
      "total_gb": 100.0,
      "used_gb": 45.2,
      "used_pct": 45.2
    }
  ]
}
```

---

### Network

```
GET /api/agents/:id/network
```

Traffic per interface (delta since last reading).

```json
{
  "agent_id": "agent-0001",
  "interfaces": [
    {
      "name": "eth0",
      "rx_bytes": 1048576,
      "tx_bytes": 524288
    }
  ]
}
```

---

### Containers

```
GET /api/agents/:id/containers
```

Docker/Podman containers detected via cgroup.

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
    }
  ]
}
```

---

### Processes

```
GET /api/agents/:id/processes
```

Top processes by CPU and memory.

```json
{
  "agent_id": "agent-0001",
  "processes": [
    {
      "pid": 1234,
      "name": "postgres",
      "cpu_pct": 12.5,
      "rss_mb": 256.0
    }
  ]
}
```

---

### GPU

```
GET /api/agents/:id/gpu
```

NVIDIA GPU stats (requires `nvidia-smi` on the agent).

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

### TCP Connections

```
GET /api/agents/:id/connections
```

TCP connections grouped by local port.

```json
{
  "agent_id": "agent-0001",
  "connections": [
    {
      "port": 443,
      "service": "https",
      "active_count": 47,
      "remote_ips": ["203.0.113.10", "198.51.100.5"]
    }
  ]
}
```

---

### Sessions

```
GET /api/agents/:id/sessions
```

Agent session info and status.

```json
{
  "agent_id": "agent-0001",
  "hostname": "web-prod-01",
  "status": "online",
  "last_seen": "2026-06-03T21:00:00+00:00"
}
```

---

### Alerts

```
GET /api/alerts
```

Alert history.

```json
{
  "alerts": [
    {
      "time": "2026-06-03T21:15:00+00:00",
      "agent_id": "agent-0001",
      "alert_type": "cpu_high",
      "message": "CPU > 90% for 3 consecutive readings (current: 94.2%)"
    }
  ]
}
```

---

### Check Update

```
GET /api/updates/check
```

Agents call this to check if an update is available.

```json
{
  "version": "0.5.0",
  "available": true,
  "sha256": "abcdef1234567890..."
}
```

---

### Download Update

```
GET /api/updates/download
```

Returns the binary file for the agent's architecture.

---

### Get Rollout Config

```
GET /api/updates/rollout
```

```json
{
  "version": "0.5.0",
  "rollout_pct": 100,
  "urgent": false
}
```

---

## Admin Endpoints

All require `Authorization: Bearer <PULSE_ADMIN_TOKEN>`.

### Rename Agent

```
PUT /api/agents/:id/name
```

```json
{"name": "web-prod-01"}
```

---

### Delete Agent

```
DELETE /api/agents/:id
```

Removes agent and its data.

```json
{"deleted": true, "agent_id": "agent-0001"}
```

---

### Execute Command

```
POST /api/agents/:id/exec
```

Execute a command on the agent via the gRPC command channel.

```json
{"command": "uptime", "args": [], "timeout_secs": 30}
```

---

### File Operations

```
POST /api/agents/:id/file
```

Read, write, or append files on the agent.

```json
{"op": "read", "path": "/etc/hostname"}
```

---

### Get Register Token

```
GET /api/register-token
```

Returns the current register token (for provisioning new agents).

```json
{"register_token": "sha256pin::secret"}
```

---

### Upload Binary

```
POST /api/updates/upload
```

Upload a new agent binary (max 50 MB). Used by CI.

Multipart form with `file` field and architecture metadata.

---

### Set Rollout

```
POST /api/updates/rollout
```

Set target version and rollout configuration.

```json
{
  "version": "0.5.0",
  "rollout_pct": 100,
  "urgent": false
}
```

---

## Examples with curl

```bash
# Health check
curl http://localhost:8080/health

# Overview
curl -s http://localhost:8080/api/overview

# List agents
curl -s http://localhost:8080/api/agents

# Latest metrics
curl -s http://localhost:8080/api/agents/agent-0001/metrics/latest

# Disks
curl -s http://localhost:8080/api/agents/agent-0001/disks

# Containers
curl -s http://localhost:8080/api/agents/agent-0001/containers

# Rename agent (admin)
curl -X PUT http://localhost:8080/api/agents/agent-0001/name \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"web-prod-01"}'

# Delete agent (admin)
curl -X DELETE http://localhost:8080/api/agents/agent-0001 \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# Get register token (admin)
curl http://localhost:8080/api/register-token \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# Upload binary (admin, from CI)
curl -X POST http://localhost:8080/api/updates/upload \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -F "file=@pulse-agent-linux-x86_64" \
  -F "arch=x86_64" \
  -F "version=0.5.0"

# Set rollout (admin)
curl -X POST http://localhost:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"version":"0.5.0","rollout_pct":100}'
```

## Examples with jq

```bash
# Online agents only
curl -s http://localhost:8080/api/agents | jq '.agents[] | select(.status == "online")'

# CPU and memory of all agents
curl -s http://localhost:8080/api/overview | jq '.agents[] | {hostname, cpu_pct, mem_pct}'

# Disks above 80%
curl -s http://localhost:8080/api/agents/agent-0001/disks | jq '.disks[] | select(.used_pct > 80)'

# Who is connecting to SSH (port 22)?
curl -s http://localhost:8080/api/agents/agent-0001/connections | jq '.connections[] | select(.service=="ssh")'

# Top process by CPU
curl -s http://localhost:8080/api/agents/agent-0001/processes | jq '.processes | sort_by(-.cpu_pct) | .[0]'
```
