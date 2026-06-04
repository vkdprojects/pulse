# Configuração

## Master (variáveis de ambiente)

O master é configurado exclusivamente por variáveis de ambiente. Na instalação via script, os defaults já funcionam — configure apenas o que precisar ajustar.

| Variável | Default | Descrição |
|---|---|---|
| `PULSE_LISTEN` | `[::]:50051` | Endereço de escuta gRPC |
| `PULSE_TLS_DIR` | `/etc/pulse` | Diretório para certs (auto-criado no boot) |
| `DATABASE_URL` | _(SQLite)_ | Se `postgres://...`, usa TimescaleDB |
| `PULSE_REGISTER_TOKEN` | _(auto-gerado)_ | Token para autenticar agents (formato `ca_pin::secret`) |
| `PULSE_RETENTION_DAYS` | `30` | Dias para manter métricas antes de deletar |
| `RUST_LOG` | `info` | Nível de log (trace, debug, info, warn, error) |

### Alertas

| Variável | Default | Descrição |
|---|---|---|
| `PULSE_ALERT_WEBHOOK` | _(desabilitado)_ | URL do webhook (Telegram/Slack/Discord/genérico) |
| `PULSE_ALERT_TELEGRAM_CHAT_ID` | — | Chat ID para Telegram |
| `PULSE_ALERT_CPU_PCT` | `90` | Threshold CPU (%) |
| `PULSE_ALERT_MEM_PCT` | `90` | Threshold memória (%) |
| `PULSE_ALERT_BW_MBPS` | `100` | Threshold bandwidth (MB/s) |

### Modificar configuração

```bash
# Via systemd override (recomendado)
sudo systemctl edit pulse-master
```

```ini
[Service]
Environment=PULSE_RETENTION_DAYS=60
Environment=PULSE_ALERT_WEBHOOK=https://api.telegram.org/bot<TOKEN>/sendMessage
Environment=PULSE_ALERT_TELEGRAM_CHAT_ID=123456
```

```bash
sudo systemctl restart pulse-master
```

## Agent (`/etc/pulse/agent.toml`)

Gerado automaticamente pelo install script. Edite apenas se precisar ajustar algo.

```toml
# Endpoint do master
master_endpoint = "https://master.example.com:50051"

# Intervalo de coleta em milissegundos
metrics_interval_ms = 10000

# Labels para agrupar/filtrar agents
labels = ["prod", "datacenter-sp"]

# Token de autenticação (formato ca_pin::secret)
auth_token = "sha256pin::secret"

# Preenchido automaticamente após primeiro registro:
[tls]
ca_cert = "/etc/pulse/certs/ca.pem"
client_cert = "/etc/pulse/certs/client.pem"
client_key = "/etc/pulse/certs/client-key.pem"
domain_name = "master"
```

### Campos

| Campo | Default | Descrição |
|---|---|---|
| `master_endpoint` | `http://[::1]:50051` | URL do master |
| `metrics_interval_ms` | `10000` | Intervalo de coleta (ms) |
| `labels` | `[]` | Tags para agrupar agents |
| `auth_token` | `""` | Token com CA pin para bootstrap |
| `tls.ca_cert` | — | CA cert (preenchido após registro) |
| `tls.client_cert` | — | Client cert (preenchido após registro) |
| `tls.client_key` | — | Client key (preenchido após registro) |
| `tls.domain_name` | `localhost` | Domain para validação TLS |
