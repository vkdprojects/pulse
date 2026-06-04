# Configuração

## Master (variáveis de ambiente)

| Variável | Obrigatória | Default | Descrição |
|---|---|---|---|
| `DATABASE_URL` | Não | _(sem DB)_ | Connection string PostgreSQL/TimescaleDB |
| `PULSE_LISTEN` | Não | `[::1]:50051` | Endereço de escuta gRPC |
| `PULSE_TLS_DIR` | Não | _(inseguro)_ | Diretório com `ca.pem`, `server.pem`, `server-key.pem` |
| `PULSE_REGISTER_TOKEN` | Não | _(aceita todos)_ | Token para autenticar agents no registro |
| `RUST_LOG` | Não | `info` | Nível de log (trace, debug, info, warn, error) |

### Alertas

| Variável | Default | Descrição |
|---|---|---|
| `PULSE_ALERT_WEBHOOK` | _(desabilitado)_ | URL do webhook (Telegram/Slack/Discord/genérico) |
| `PULSE_ALERT_TELEGRAM_CHAT_ID` | — | Chat ID para Telegram |
| `PULSE_ALERT_CPU_PCT` | `90` | Threshold CPU (%) para alerta |
| `PULSE_ALERT_MEM_PCT` | `90` | Threshold memória (%) para alerta |
| `PULSE_ALERT_BW_MBPS` | `100` | Threshold bandwidth (MB/s) para alerta |

## Agent (`/etc/pulse-agent.toml`)

```toml
# Endpoint do master
master_endpoint = "https://master.example.com:50051"

# Intervalo de coleta em milissegundos
metrics_interval_ms = 10000

# Labels para agrupar/filtrar agents
labels = ["prod", "datacenter-sp"]

# Token de autenticação (deve bater com PULSE_REGISTER_TOKEN do master)
auth_token = "meu-token-secreto"

# Configuração TLS (omitir para conexão insegura)
[tls]
ca_cert = "/etc/pulse/ca.pem"
client_cert = "/etc/pulse/agent.pem"
client_key = "/etc/pulse/agent-key.pem"
domain_name = "master.example.com"
```

### Comportamento sem config

Se `/etc/pulse-agent.toml` não existir, usa defaults:
- Conecta em `http://[::1]:50051` (localhost, inseguro)
- Intervalo de 10 segundos
- Sem labels, sem auth token
