# ⚡ Vixpi Pulse

> **O monitoring agent que cabe no bolso do seu servidor.**

Binário de **1.9 MB**. Roda com **2-5 MB de RAM**. Coleta tudo. Executa comandos remotos. Zero dependências.

```
┌─────────────┐    gRPC/mTLS     ┌──────────────┐    ┌─────────────┐    ┌─────────┐
│ pulse-agent │ ═══════════════▶ │ pulse-master │ ══▶│ TimescaleDB │ ══▶│ Grafana │
│  (1.9 MB)   │ ◀═══════════════ │  (central)   │    └─────────────┘    └─────────┘
└─────────────┘   commands        └──────────────┘
     × N agents                        auto-provisioning
```

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

## 🚀 Quick Start (30 segundos)

```bash
# Sobe tudo: DB + Master + 3 Agents + Grafana
docker compose up --build

# Dashboard → http://localhost:3000 (user: admin, pass: pulse)
# Métricas já aparecendo em tempo real.
```

**Ou valide com o teste automatizado:**

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

## 📦 Deploy em Produção

```bash
# Build estático
cargo build --release -p pulse-agent --target x86_64-unknown-linux-musl

# Instala no server
scp target/x86_64-unknown-linux-musl/release/pulse-agent server:/tmp/
ssh server 'sudo /tmp/install.sh /tmp/pulse-agent'
ssh server 'sudo systemctl enable --now pulse-agent'
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

**MIT License** · Made with ❤️ by GCN Soluções
