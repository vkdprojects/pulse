# Funcionalidades

## 📊 Métricas Coletadas

### CPU
| Métrica | Detalhe |
|---|---|
| Uso total (%) | Delta entre leituras de `/proc/stat`, inclui iowait como idle |
| Uso por core | Mesmo cálculo, por CPU individual |
| Load average 1/5/15m | De `/proc/loadavg`, centipercent (1.50 → 150) |

### Memória
| Métrica | Detalhe |
|---|---|
| Total | MemTotal de `/proc/meminfo` |
| Usada | Total - Available - ZFS ARC |
| Disponível | MemAvailable (ou MemFree se indisponível) |
| Swap total/usado | SwapTotal - SwapFree |
| Cgroup limit | `/sys/fs/cgroup/memory.max` (v2) ou `memory.limit_in_bytes` (v1) |
| Cgroup usage | `/sys/fs/cgroup/memory.current` (v2) ou `memory.usage_in_bytes` (v1) |
| ZFS ARC | Subtraído do "usado" (memória reclaimável) |

### Disco
| Métrica | Detalhe |
|---|---|
| Uso (total/used) | `statvfs` syscall por mount point (ext4, xfs, btrfs, zfs, f2fs) |
| I/O read bytes | Delta de `/proc/diskstats` (sectors * 512) |
| I/O write bytes | Delta de `/proc/diskstats` |
| I/O read ops | Delta de operações de leitura |
| I/O write ops | Delta de operações de escrita |
| Health (model, temp) | `/sys/block/*/device/model` + hwmon |

### Rede
| Métrica | Detalhe |
|---|---|
| RX bytes (por intervalo) | Delta de `/proc/net/dev` |
| TX bytes (por intervalo) | Delta de `/proc/net/dev` |
| RX/TX packets | Delta |
| RX/TX errors | Acumulado (counter semântico) |

### Docker / Podman
| Métrica | Detalhe |
|---|---|
| Container memory usage | Cgroup v2: `memory.current` |
| Container memory limit | Cgroup v2: `memory.max` |
| Container ID + state | Leitura de cgroup path names |
| Podman rootless | Detecta `user.slice` com prefix `libpod-` |

### GPU (NVIDIA)
| Métrica | Detalhe |
|---|---|
| Utilização (%) | `nvidia-smi --query-gpu` |
| Memória (used/total) | Em bytes |
| Temperatura | °C |
| Power draw | Watts |

### Sistema
| Métrica | Detalhe |
|---|---|
| Uptime | `/proc/uptime` em segundos |
| Processos total | `/proc/stat` field `processes` |
| Processos running | `/proc/stat` field `procs_running` |
| Top 5 processos | `/proc/[pid]/stat` — ordenados por CPU time + RSS |
| Temperatura | `/sys/class/thermal/thermal_zone*/temp` (millicelsius) |
| Bateria | `/sys/class/power_supply/BAT0/capacity` (%) |

### SSH Sessions
| Métrica | Detalhe |
|---|---|
| Usuário logado | Parsing binário de `/var/run/utmp` |
| IP de origem | Campo `ut_host` do struct utmpx |
| Terminal (pts/N) | Campo `ut_line` |
| PID do sshd | Campo `ut_pid` |

### Conexões TCP (Netflow leve)
| Métrica | Detalhe |
|---|---|
| Porta local | Portas com conexões ESTABLISHED |
| Serviço | Auto-detect: ssh(22), http(80), https(443), postgres(5432), mysql(3306), redis(6379), mongodb(27017) |
| Contagem | Número de conexões ativas por porta |
| IPs remotos | Lista de IPs únicos conectados (max 50/porta) |
| Fonte | `/proc/net/tcp` + `/proc/net/tcp6` |
| Filtro | Ignora localhost, só ESTABLISHED (st=01) |

---

## 🎮 Comandos Remotos

### Exec
Executa qualquer programa no agent com streaming de output.

- Programa + argumentos + variáveis de ambiente
- Working directory customizável
- Timeout configurável (default 300s)
- Kill automático ao timeout
- stdout/stderr streamed em real-time via gRPC

### Package
Gerenciamento de pacotes via apt-get.

- `install` — instala pacotes
- `remove` — remove pacotes
- `update` — atualiza lista de pacotes

### File
Operações de arquivo remoto.

- `read` — lê conteúdo de arquivo (retorna via stream)
- `write` — cria/sobrescreve arquivo
- `append` — adiciona ao final do arquivo

### Log Tail
Streaming de logs em tempo real.

- Equivalente a `tail -n N [-f] /path/to/log`
- Configura número de linhas iniciais
- Modo follow: stream contínuo até timeout

---

## 🔔 Alertas

### Thresholds
| Alerta | Trigger | Config (env var) |
|---|---|---|
| CPU alta | >90% por 3 leituras consecutivas | `PULSE_ALERT_CPU_PCT` |
| Memória alta | >90% por 3 leituras consecutivas | `PULSE_ALERT_MEM_PCT` |
| Bandwidth alta | >100 MB/s por 3 leituras | `PULSE_ALERT_BW_MBPS` |
| Agent offline | Sem métricas por >45 segundos | Automático |

### Canais de notificação
| Canal | Detecção automática |
|---|---|
| Telegram | URL contém `api.telegram.org` |
| Slack | URL contém `hooks.slack.com` |
| Discord | URL contém `discord.com/api/webhooks` |
| Genérico | Qualquer outra URL (JSON com timestamp) |

---

## 🔐 Segurança

- **TLS automático**: certs gerados no primeiro boot do master
- **CA pin no token**: previne ataques MITM no bootstrap
- **Client certs automáticos**: emitidos no Register, usados em boots subsequentes
- **mTLS completo**: após primeiro registro, todas as conexões usam mutual TLS
- **Auth token**: impede registro não autorizado
- **Systemd hardening**: NoNewPrivileges, ProtectSystem=strict
- **Zero unsafe**: exceto 1 syscall (statvfs)
- **Zero vulnerabilidades**: `cargo audit` limpo

---

## 🔄 Auto-Update

### Fluxo
1. CI compila e faz upload de binários ao master (`POST /api/updates/upload`)
2. Master serve binários + controla rollout
3. Agents verificam a cada 5 minutos (`GET /api/updates/check`)
4. Download + SHA256 verify + atomic self-replace + restart via systemd

### Rollout gradual
| Config | Descrição |
|---|---|
| `rollout_pct = 100` | Todos os agents (default) |
| `rollout_pct = 10` | 10% primeiro (canary) |
| `urgent = true` | Sem jitter, update imediato |

### Rollback automático
- Agent salva `.prev` antes de substituir
- Marker file conta tentativas de boot
- 3 crashes → restaura binário anterior automaticamente

### Segurança
- SHA256 verificado antes de aplicar
- Path sanitizado (sem traversal)
- Binários servidos pelo master (agents não precisam de GitHub)
- Upload protegido por `PULSE_ADMIN_TOKEN`

---

## 💾 Persistência

### SQLite (default)
- Zero config, zero dependências
- Database criada automaticamente no boot
- Retention worker deleta dados > `PULSE_RETENTION_DAYS` (default 30)

### TimescaleDB (opcional, via `DATABASE_URL`)
- Hypertables: auto-partição por tempo
- Compressão: chunks >7 dias comprimidos (~90% redução)
- Retention: configurável
- Continuous aggregates: médias horárias pré-computadas
- Integração nativa com Grafana

---

## 🏥 Operações

- **Health endpoint**: `GET :8080/health` → `{"status":"ok"}`
- **Reconnect**: exponential backoff (1s → 60s)
- **Offline buffer**: ring buffer de 100 métricas sobrevive desconexões
- **Graceful shutdown**: SIGINT/SIGTERM fecha conexões corretamente
- **Retention automática**: worker background deleta dados antigos
