# ⚡ Vixpi Pulse

> **O monitoring agent que cabe no bolso do seu servidor.**

Binário de **1.9 MB**. Roda com **2-5 MB de RAM**. Coleta tudo. Executa comandos remotos. **Zero dependências.**

```
┌─────────────┐    gRPC/mTLS     ┌──────────────┐
│ pulse-agent │ ═══════════════▶ │ pulse-master │ ──▶ REST API :8080
│  (1.9 MB)   │ ◀═══════════════ │  (~4 MB)     │ ──▶ SQLite embedded
└─────────────┘   commands        └──────────────┘     (zero config)
     × N agents                    auto-provisioning
```

---

## 🚀 Quick Start — 2 comandos, zero dependências

```bash
# ══════ No servidor central (master): ══════
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | sudo sh -

# ══════ Em cada servidor que quer monitorar: ══════
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | \
  PULSE_MASTER=https://SEU-MASTER-IP:50051 PULSE_TOKEN=token-mostrado-no-install sudo sh -
```

**Pronto.** Abra `http://master:8080/api/agents` — seus servidores já aparecem.

> Sem Docker. Sem Postgres. Sem Grafana. Sem config files.
> O master gera TLS, token e database automaticamente no primeiro boot.

---

## 🖥️ Pulse TUI — Dashboard no Terminal

```bash
# Instala na sua máquina (macOS / Linux, sem sudo)
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | INSTALL_COMPONENT=cli sh -

# Conecta ao master
pulse http://master:8080
```

```
⚡ Pulse Fleet                                    ↻ 5s │ 4 agents │ 4 online
────────────────────────────────────────────────────────────────────────────────
 ● UP │ web-prod-01     │ CPU 23% │ MEM 67% │ ↑12d 4h │ v0.3.0
 ● UP │ db-master       │ CPU 12% │ MEM 89% │ ↑ 5d 1h │ v0.3.0
 ○ DN │ worker-03       │ —       │ —       │ offline │ v0.2.0
 ● UP │ cache-01        │ CPU  3% │ MEM 34% │ ↑42d 7h │ v0.3.0
────────────────────────────────────────────────────────────────────────────────
 [j/k] navigate  [Enter] detail  [/] filter  [u] update  [e] exec  [q] quit
```

Binário de **1.7 MB**. Roda em qualquer terminal. Atualização em tempo real a cada 5s.

---

## 🔄 Auto-Update — Zero SSH para atualizar agents

```bash
# Push tag → CI compila → agents se atualizam sozinhos
git tag v0.4.0 && git push --tags

# Ou manualmente: upload + rollout
curl -X POST http://master:8080/api/updates/upload -H "Authorization: Bearer $TOKEN" -F "version=v0.4.0" -F "arch=x86_64" -F "binary=@pulse-agent"
curl -X POST http://master:8080/api/updates/rollout -H "Authorization: Bearer $TOKEN" -d '{"version":"v0.4.0","rollout_pct":100}'
```

- Agents verificam a cada 5 min, baixam do master, verificam SHA256
- Rollout gradual (10% → 50% → 100%)
- Rollback automático se o novo binário crashar 3x
- Zero GitHub tokens nos agents — tudo pelo master

---

## 🏆 vs Concorrência

| | **Vixpi Pulse** | Beszel | Netdata | Telegraf | Zabbix Agent |
|---|---|---|---|---|---|
| **RAM idle** | **2-5 MB** 🥇 | 15-23 MB | 100-200 MB | 50-100 MB | 50-128 MB |
| **Binário** | **1.9 MB** 🥇 | ~10 MB | ~100 MB | ~50 MB | ~15 MB |
| **Linguagem** | Rust (no GC) | Go | C | Go | Go |
| **Remote exec** | ✅ streaming | ❌ | ❌ | ❌ | limitado |
| **Log tail remoto** | ✅ `tail -f` | ❌ | ❌ | ❌ | ❌ |
| **File ops remoto** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Package mgmt** | ✅ apt | ❌ | ❌ | ❌ | ❌ |
| **mTLS automático** | ✅ zero-config | SSH | HTTPS | manual | manual |
| **Auto-update** | ✅ gradual rollout | cron | kickstart.sh | ❌ | ❌ |
| **TUI nativa** | ✅ 1.7 MB | ❌ | ❌ | ❌ | ❌ |
| **Docker stats** | ✅ | ✅ | ✅ | plugin | plugin |
| **GPU (nvidia)** | ✅ | ✅ | ✅ | plugin | ❌ |
| **Cgroup aware** | ✅ v1+v2 | ❌ | ✅ | ❌ | ❌ |
| **Licença** | MIT | MIT | GPL-3 | MIT | GPL-2 |

> **TL;DR:** 10x mais leve que Beszel, com remote execution que nenhum concorrente leve tem.

---

## 📊 O Que Coleta

| Métrica | Fonte | Nota |
|---|---|---|
| CPU (per-core + total) | `/proc/stat` | Basis points (0-10000) |
| Load average (1/5/15m) | `/proc/loadavg` | |
| Memory + swap | `/proc/meminfo` | |
| Cgroup memory | `/sys/fs/cgroup/` | v1 + v2 auto-detect |
| Disk usage | `statvfs` | ext4, xfs, btrfs, zfs |
| Disk I/O | `/proc/diskstats` | read/write bytes + ops |
| Disk health | `/sys/block/*/device/` | model + temp |
| Network | `/proc/net/dev` | per interface |
| Docker containers | cgroup | CPU + mem por container |
| GPU | `nvidia-smi` | util%, mem, temp, power |
| Temperature | `/sys/class/thermal/` | todas as zones |
| Top 5 processes | `/proc/[pid]/stat` | por CPU + RSS |
| SSH sessions | `/var/run/utmp` | user, IP, tty, pid |
| Battery | `/sys/class/power_supply/` | % |
| System | `/proc/uptime` | uptime + process count |
| TCP Connections | `/proc/net/tcp{,6}` | IPs remotos por porta (netflow leve) |

---

## 🎮 Comandos Remotos (killer feature)

```
Master → Agent: "roda isso aí"
Agent → Master: stdout/stderr em tempo real
```

- **Exec** — roda qualquer programa com args, env, working dir, timeout
- **Package** — `apt install/remove/update` pacotes
- **File** — read/write/append arquivos remotos
- **Log tail** — `tail -f /var/log/syslog` streaming em tempo real

Tudo via gRPC bidirecional. Tudo com timeout + kill automático.

---

## 🔐 Segurança

- **mTLS em tudo** — certificado mútuo, zero conexão não autenticada
- **Auto-provisioning** — agent registra, master gera e envia cert na hora
- **Zero unsafe** no workspace (exceto 1 syscall de statvfs)
- **cargo audit** — 0 vulnerabilidades
- **Systemd hardening** — NoNewPrivileges, ProtectSystem=strict

---

## 🔔 Alerting

```bash
# Config via env vars no master:
PULSE_ALERT_WEBHOOK=https://api.telegram.org/bot<TOKEN>/sendMessage
PULSE_ALERT_TELEGRAM_CHAT_ID=123456
PULSE_ALERT_CPU_PCT=90   # dispara se CPU > 90% por 3 leituras
PULSE_ALERT_MEM_PCT=90   # dispara se MEM > 90% por 3 leituras
```

Auto-detecta formato: **Telegram**, **Slack**, **Discord**, ou webhook genérico.

---

## 🌐 REST API (21 endpoints)

Base: `http://master:8080`

| Endpoint | Descrição |
|---|---|
| `GET /health` | Health check |
| `GET /api/ca` | CA certificate (PEM) |
| `GET /api/overview` | Dashboard summary (todos agents + stats) |
| `GET /api/alerts` | Histórico de alertas |
| `GET /api/agents` | Listar agents |
| `GET /api/agents/:id/metrics` | Últimas 20 métricas |
| `GET /api/agents/:id/metrics/latest` | Snapshot atual (1 leitura) |
| `GET /api/agents/:id/disks` | Uso de disco por mountpoint |
| `GET /api/agents/:id/network` | Tráfego por interface |
| `GET /api/agents/:id/containers` | Docker containers |
| `GET /api/agents/:id/processes` | Top processos |
| `GET /api/agents/:id/gpu` | GPU stats |
| `GET /api/agents/:id/connections` | Conexões TCP por porta (netflow) |
| `GET /api/agents/:id/sessions` | Info do agent |
| `DELETE /api/agents/:id` | Remover agent 🔒 |
| `POST /api/agents/:id/exec` | Executar comando 🔒 |
| `GET /api/updates/check` | Verifica update disponível |
| `GET /api/updates/download` | Download do binário |
| `POST /api/updates/upload` | Upload de binário (CI) 🔒 |
| `GET /api/updates/rollout` | Ver config de rollout |
| `POST /api/updates/rollout` | Setar target version 🔒 |

🔒 = requer `Authorization: Bearer PULSE_ADMIN_TOKEN`

```bash
# Exemplo: overview de todos os servidores
curl -s http://master:8080/api/overview | jq

# Exemplo: quem está acessando meu servidor na porta 22?
curl -s http://master:8080/api/agents/agent-0001/connections | jq '.connections[] | select(.service=="ssh")'
```

---

## 🔄 Update / Upgrade

Rode o mesmo comando de instalação novamente. O script para o serviço, substitui o binário e reinicia:

```bash
# Update master:
curl -sfL .../install-pulse.sh | sudo sh -

# Update agent:
curl -sfL .../install-pulse.sh | PULSE_MASTER=https://x:50051 sudo sh -
```

Migrations rodam automaticamente no boot. Agents reconectam sozinhos.

**Versão específica:**
```bash
VERSION=v0.2.0 curl -sfL .../install-pulse.sh | sudo sh -
```

---

## 📦 Scaling (100+ agents)

Por default o master usa **SQLite embedded** (zero config, ~2 MB RAM). Para escalar:

```bash
# Adicione TimescaleDB quando precisar:
systemctl stop pulse-master

# Edite o service pra adicionar DATABASE_URL:
systemctl edit pulse-master
# [Service]
# Environment=DATABASE_URL=postgres://pulse:senha@localhost:5432/pulse

systemctl start pulse-master
# Master detecta postgres:// → usa TimescaleDB com hypertables, compression, etc.
```

Com TimescaleDB você ganha: compression 90%+, continuous aggregates, Grafana nativo, retenção configurável.

---

## 🐳 Demo Mode (Docker Compose + Grafana)

```bash
# Sobe tudo: DB + Master + 3 Agents + Grafana
docker compose up --build

# Dashboard → http://localhost:3000 (user: admin, pass: pulse)
```

Útil para: demo rápida, desenvolvimento local, ou se quiser Grafana com dashboards prontos.

---

## 🛠️ Dev Environment

```bash
# Pré-requisitos: Rust 1.88+, protoc
brew install protobuf just   # macOS

# Build
cargo build --workspace

# Rodar master (SQLite, zero config):
cargo run -p pulse-master

# Rodar agent:
cargo run -p pulse-agent -- .certs/agent.toml

# Quality checks:
just ci   # fmt + clippy + test
```

---

## 📁 Gerenciamento

```bash
# Status
systemctl status pulse-master
systemctl status pulse-agent

# Logs em tempo real
journalctl -u pulse-master -f
journalctl -u pulse-agent -f

# Reiniciar
systemctl restart pulse-master

# Desinstalar
systemctl disable --now pulse-master
rm /usr/local/bin/pulse-master
rm -rf /var/lib/pulse /etc/pulse
```

---

## 📈 Números

| Métrica | Valor |
|---|---|
| Binário agent | **3.1 MB** |
| Binário master | **~5.8 MB** |
| Binário CLI (TUI) | **1.7 MB** |
| RAM idle (agent) | **2-5 MB** |
| RAM idle (master) | **~5 MB** |
| Dependências externas | **0** |
| Tempo de install | **< 5 segundos** |
| Tempo até primeiro dado | **~1s após boot** |
| Testes automatizados | 51 |
| Vulnerabilidades | 0 |
| Unsafe code | 0 |

---

## 🗂️ Estrutura

```
crates/
├── pulse-proto/      # Protobuf (gRPC contracts)
├── pulse-agent/      # O daemon (3.1 MB)
│   ├── main.rs       # connect → register → stream (reconnect + backoff)
│   ├── config.rs     # TOML config
│   ├── telemetry.rs  # /proc + cgroup + statvfs + nvidia-smi
│   ├── containers.rs # Docker stats via cgroup
│   ├── worker.rs     # exec/package/file/log-tail
│   └── updater.rs    # auto-update (check + download + self-replace + rollback)
├── pulse-master/     # Servidor central (~5.8 MB)
│   ├── service.rs    # gRPC + auto-provisioning
│   ├── storage.rs    # SQLite (default) ou Postgres (via DATABASE_URL)
│   ├── alerts.rs     # Threshold → webhook
│   ├── api.rs        # REST API (axum)
│   ├── auth.rs       # Bearer token middleware (PULSE_ADMIN_TOKEN)
│   ├── updates.rs    # Binary server + rollout (auto-update)
│   └── ca.rs         # X.509 CA (rcgen)
├── pulse-cli/        # TUI interativa (1.7 MB, bin: "pulse")
│   ├── app.rs        # State + navigation + key handling
│   ├── client.rs     # REST client (reqwest)
│   ├── event.rs      # Async event stream (crossterm + tokio)
│   └── ui.rs         # ratatui rendering (fleet, detail, update, exec)
└── pulse-tests/      # Integration tests
```

---

## 📖 Documentação Completa

→ **[docs/](./docs/)** — Wiki navegável pelo GitHub

---

**MIT License** · Made with ❤️ by GCN Soluções
