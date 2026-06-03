# Vixpi Pulse — Roadmap & Gaps

## Status Atual: ~85% production-ready

O core funciona, testes passam, e2e Docker valida. Mas faltam **4 pontos críticos de operação** e algumas features de paridade.

---

## 🔴 Crítico — Sem isso, não vai pra produção real

### 1. Agent Offline Detection (heartbeat)

**Problema:** Se um server morre ou o agent crasheia, o master não sabe. Ninguém é notificado.

**O que precisa:**
- Master mantém timestamp do último `MetricsPayload` recebido de cada agent
- Background task checa a cada 30s: `last_seen_at < now() - 3*interval` → agent considerado offline
- Dispara alerta (webhook) com tipo `agent_offline`
- Quando agent reconecta, dispara `agent_online`

**Onde implementar:**
- `storage.rs`: query `SELECT id, last_seen_at FROM agents WHERE last_seen_at < now() - interval '30 seconds'`
- `alerts.rs`: novo tipo de alerta `agent_offline`/`agent_online`
- `main.rs` do master: spawn background task

**Esforço:** 2-3h

---

### 2. Auth no Registro (pre-shared key)

**Problema:** Qualquer processo na rede pode chamar `Register()` e se passar por um agent legítimo. Zero autenticação no primeiro contato.

**O que precisa:**
- Master configura um `PULSE_REGISTER_TOKEN` (env var)
- Agent envia esse token no `RegisterRequest` (novo campo `auth_token`)
- Master rejeita com `Status::Unauthenticated` se token não bate
- Sem token configurado no master = aceita qualquer um (dev mode)

**Onde implementar:**
- `pulse.proto`: adicionar `string auth_token = 6` no `RegisterRequest`
- `service.rs`: validar antes de gerar cert
- `config.rs` do agent: novo campo `auth_token`

**Esforço:** 1h

---

### 3. Buffering Offline no Agent

**Problema:** Se o master fica offline (deploy, crash, rede), o agent perde todas as métricas desse período. Quando reconecta, só envia dados novos.

**O que precisa:**
- Ring buffer local no agent (em memória, bounded — ex: últimas 100 métricas)
- Quando `metrics_tx.send()` falha (channel fechado / master offline), armazena no buffer
- Ao reconectar, drena o buffer antes de continuar streaming normal
- Config: `buffer_size = 100` no TOML (default)

**Onde implementar:**
- `main.rs` do agent: `VecDeque<MetricsPayload>` com capacidade fixa
- No loop de reconexão: envia buffer acumulado primeiro

**Esforço:** 2h

---

### 4. Health Endpoint no Master

**Problema:** Em produção o master fica atrás de load balancer ou Kubernetes. Sem `/health`, o LB não sabe se o master está pronto.

**O que precisa:**
- HTTP endpoint `GET /health` que retorna `200 OK` + JSON com status
- Verifica: DB conectada? CA carregada? gRPC listening?
- Pode ser via tonic-health (gRPC health checking protocol) ou HTTP simples separado

**Onde implementar:**
- Opção A: `tonic-health` crate (padrão gRPC)
- Opção B: tokio TCP listener separado na porta 8080 com HTTP mínimo

**Esforço:** 1-2h

---

## 🟡 Importante — Paridade com concorrentes

### 5. Bandwidth Alerting

**O que falta:** Alertar quando network rx/tx ultrapassa threshold. Hoje só alertamos CPU e memória.

**Esforço:** 1h (lógica já existe, só adicionar threshold para net)

---

### 6. REST API no Master

**Problema:** Não há como consultar dados programaticamente. Sem API, não integra com nada externo (scripts, dashboards custom, automações).

**O que precisa:**
- `GET /api/agents` — lista agents com status
- `GET /api/agents/:id/metrics?from=&to=` — consulta métricas
- `POST /api/agents/:id/command` — envia comando ao agent
- Autenticação via API key simples

**Esforço:** 4-6h (usar axum que já vem com tonic)

---

### 7. Agent Status no DB

**Problema:** O campo `last_seen_at` existe, mas não temos um campo `status` (online/offline) nem histórico de status.

**O que precisa:**
- Coluna `status TEXT` na tabela agents (`online`/`offline`)
- Update pelo background task de heartbeat
- Histórico opcional: tabela `agent_events(time, agent_id, event_type)`

**Esforço:** 1h

---

### 8. Podman Support

**Problema:** Nosso containers.rs procura por paths Docker-specific. Quem usa Podman rootless tem paths diferentes.

**O que precisa:**
- Detectar se é Podman (path: `$XDG_RUNTIME_DIR/containers/` ou `/sys/fs/cgroup/user.slice/`)
- Ler cgroups do Podman da mesma forma

**Esforço:** 2h

---

## 🟢 Nice to Have — Polish e adoção

### 9. Mais Notification Providers

Beszel suporta 20+. Nós suportamos 3 (Telegram, Slack, Discord via webhook genérico).

**Faltam providers populares:**
- Ntfy (self-hosted push)
- PagerDuty / OpsGenie
- Email (SMTP)
- Microsoft Teams
- Matrix

**Esforço:** 1h por provider (são só templates de JSON/HTTP)

---

### 10. Battery Monitoring

**O que falta:** Ler `/sys/class/power_supply/BAT*/capacity` para laptops/UPS.

**Esforço:** 30min

---

### 11. ZFS ARC Memory

**O que falta:** No cálculo de memória, ZFS ARC é contada como "used" mas é liberável. Beszel separa isso.

**O que precisa:** Ler `/proc/spl/kstat/zfs/arcstats` se existir, subtrair do used.

**Esforço:** 30min

---

### 12. Continuous Aggregates no TimescaleDB

**O que falta:** Pré-computar médias horárias/diárias para dashboards rápidos em ranges longos.

**O que precisa:**
```sql
CREATE MATERIALIZED VIEW metrics_hourly
WITH (timescaledb.continuous) AS
SELECT time_bucket('1 hour', time) AS bucket, agent_id,
       avg(cpu_total_bp) AS avg_cpu, max(cpu_total_bp) AS max_cpu,
       avg(mem_used) AS avg_mem
FROM metrics
GROUP BY bucket, agent_id;
```

**Esforço:** 1h (só SQL na migration)

---

## Prioridade de Implementação

```
Semana 1 (crítico):
  [1] Agent offline detection     — 2-3h
  [2] Auth no registro            — 1h
  [3] Buffering offline           — 2h
  [4] Health endpoint             — 1-2h

Semana 2 (paridade):
  [5] Bandwidth alerting          — 1h
  [6] REST API                    — 4-6h
  [7] Agent status no DB          — 1h

Semana 3 (polish):
  [8] Podman support              — 2h
  [9] Mais notif providers        — 3h
  [10-12] Battery, ZFS, ContinAgg — 2h
```

**Total estimado para 100%:** ~20-25h de trabalho.
