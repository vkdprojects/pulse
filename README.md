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
| **Docker stats** | ✅ | ✅ | ✅ | plugin | plugin |
| **GPU (nvidia)** | ✅ | ✅ | ✅ | plugin | ❌ |
| **Cgroup aware** | ✅ v1+v2 | ❌ | ✅ | ❌ | ❌ |
| **Licença** | MIT | MIT | GPL-3 | MIT | GPL-2 |

> **TL;DR:** 10x mais leve que Beszel, com remote execution que nenhum concorrente leve tem.

---

## 🐳 Demo Mode (Docker Compose + Grafana)

```bash
# Sobe tudo: DB + Master + 3 Agents + Grafana
docker compose up --build

# Dashboard → http://localhost:3000 (user: admin, pass: pulse)
# Métricas já aparecendo em tempo real.
```

**Validar que está funcionando (espere ~15s após subir):**

```bash
# API: ver agents conectados
curl http://localhost:8080/api/agents

# API: ver métricas do agent-0001
curl http://localhost:8080/api/agents/agent-0001/metrics

# Grafana: http://localhost:3000 (admin/pulse)
```

**Ou rode o teste automatizado:**

```bash
./tests/e2e-docker.sh
```

```
✓ 3 agents registered in DB
✓ Metrics persisted in TimescaleDB
✓ CPU telemetry data present
✓ Memory telemetry data present
✓ mTLS connection successful
✓ Multiple agents sending distinct metrics
✓ TimescaleDB hypertables active
ALL 7 CHECKS PASSED ✓
```

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
| System | `/proc/uptime` | uptime + process count |

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

## 🛠️ Dev Environment

```bash
just --list        # ver todos os comandos

just dev-up        # sobe TimescaleDB local
just dev-master    # master com DB + mTLS
just dev-agent     # agent conecta

just ci            # fmt + clippy + test
just docker-test   # e2e completo no Docker
```

---

## 🔄 Update / Upgrade

**Master:**

```bash
# 1. Build nova versão
cargo build --release -p pulse-master --target x86_64-unknown-linux-musl

# 2. Substitui binário e reinicia
scp target/x86_64-unknown-linux-musl/release/pulse-master server:/tmp/
ssh server 'sudo systemctl stop pulse-master && sudo cp /tmp/pulse-master /usr/local/bin/ && sudo systemctl start pulse-master'
```

O master roda migrations automaticamente no boot (`Storage::migrate()`). Novas tabelas são criadas com `IF NOT EXISTS` e colunas novas via `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`. **Não precisa rodar SQL manual.**

**Agent:**

```bash
# 1. Build nova versão
cargo build --release -p pulse-agent --target x86_64-unknown-linux-musl

# 2. Substitui binário e reinicia
scp target/x86_64-unknown-linux-musl/release/pulse-agent server:/tmp/
ssh server 'sudo systemctl stop pulse-agent && sudo cp /tmp/pulse-agent /usr/local/bin/ && sudo systemctl start pulse-agent'
```

O agent reconecta automaticamente com backoff exponencial. Não precisa de coordenação com o master — atualize em qualquer ordem.

**Docker:**

```bash
docker compose pull   # ou rebuild se custom
docker compose up -d --build
```

Migrations rodam no boot do master. Agents reconectam sozinhos.

**Ordem recomendada:** master primeiro (para ter o schema novo antes de receber dados novos), depois agents. Mas funciona em qualquer ordem — campos novos do agent são ignorados silenciosamente pelo master antigo, e o master novo aceita payloads de agents antigos (campos opcionais no proto).

---

## 📦 Deploy em Produção

### Master (one-liner)

```bash
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | \
  INSTALL_COMPONENT=master \
  PULSE_DB_URL="postgres://pulse:senha@localhost:5432/pulse" \
  sudo sh -
```

Isso faz tudo: baixa o binário (~3 MB), gera certs mTLS, cria systemd service, inicia.

> **Pré-requisito:** TimescaleDB rodando. Para instalar rápido:
> `docker run -d --name tsdb -p 5432:5432 -e POSTGRES_PASSWORD=senha -e POSTGRES_USER=pulse -e POSTGRES_DB=pulse timescale/timescaledb:latest-pg17`

### Agent (one-liner)

```bash
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | \
  PULSE_MASTER="https://SEU-MASTER:50051" \
  PULSE_TOKEN="seu-token" \
  sudo sh -
```

**1.9 MB baixado → rodando em <5 segundos.** Detecta x86_64 ou aarch64 automaticamente.

### Repositório privado

Se o repo é privado, passe um GitHub token com acesso a releases:

```bash
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | \
  GITHUB_TOKEN="ghp_xxxxx" \
  PULSE_MASTER="https://master:50051" \
  sudo sh -
```

### Versão específica

```bash
VERSION=v0.2.0 curl -sfL .../install-pulse.sh | sudo sh -
```

### Gerenciamento

```bash
systemctl status pulse-agent    # ver status
journalctl -u pulse-agent -f    # logs em tempo real
systemctl restart pulse-agent   # reiniciar
```

---

## 🗂️ Estrutura

```
crates/
├── pulse-proto/      # Protobuf (gRPC contracts)
├── pulse-agent/      # O daemon (1.9 MB)
│   ├── main.rs       # connect → register → stream (reconnect + backoff)
│   ├── config.rs     # TOML config
│   ├── telemetry.rs  # /proc + cgroup + statvfs + nvidia-smi
│   ├── containers.rs # Docker stats via cgroup
│   └── worker.rs     # exec/package/file/log-tail
├── pulse-master/     # Servidor central
│   ├── service.rs    # gRPC + auto-provisioning
│   ├── storage.rs    # TimescaleDB (sqlx)
│   ├── alerts.rs     # Threshold → webhook
│   └── ca.rs         # X.509 CA (rcgen)
└── pulse-tests/      # Integration tests
```

---

## 📈 Números

| Métrica | Valor |
|---|---|
| Binário agent | **1.9 MB** |
| RAM idle (agent) | **2-5 MB** |
| Testes automatizados | 13 |
| Checks e2e Docker | 7 |
| Vulnerabilidades | 0 |
| Unsafe code | 0 |
| Tempo até primeiro dado | ~1s após boot |

---

## 📖 Documentação Completa

→ **[docs/](./docs/)** — Wiki completa navegável pelo GitHub

- [Instalação do Master](./docs/master-install.md) — TimescaleDB + systemd + TLS
- [Instalação do Agent](./docs/agent-install.md) — Deploy em cada servidor
- [Configuração](./docs/configuration.md) — Todas as env vars e campos TOML
- [Funcionalidades](./docs/features.md) — Lista 1-a-1 de tudo que coleta
- [REST API](./docs/api.md) — Endpoints HTTP para consultar agents e métricas
- [Alertas](./docs/alerting.md) — Telegram, Slack, Discord, webhooks
- [Segurança](./docs/security.md) — mTLS, auth token, hardening
- [Docker](./docs/docker.md) — Dev environment + queries SQL úteis
- [Grafana](./docs/grafana.md) — Setup + queries customizadas
- [Troubleshooting](./docs/troubleshooting.md) — Problemas comuns

---

**MIT License** · Made with ❤️ by GCN Soluções
