# Instalação do Agent

O `pulse-agent` é o daemon que roda em cada servidor monitorado. Binário de 1.9 MB, 2-5 MB RAM.

## Pré-requisitos

- Linux x86_64 ou aarch64
- Acesso de rede ao master (porta 50051)

## 1. Instalar o binário

```bash
# Copiar do master ou baixar do CI
scp master:/usr/local/bin/pulse-agent /tmp/

# Instalar
sudo cp /tmp/pulse-agent /usr/local/bin/
sudo chmod +x /usr/local/bin/pulse-agent
```

## 2. Copiar certificados TLS

```bash
sudo mkdir -p /etc/pulse

# Copiar do master (gerados com pulse-master gen-certs)
scp master:/etc/pulse/ca.pem /etc/pulse/
scp master:/etc/pulse/client.pem /etc/pulse/agent.pem
scp master:/etc/pulse/client-key.pem /etc/pulse/agent-key.pem
```

> **Nota:** Em produção com auto-provisioning, os agents recebem certificados automaticamente no registro. Os certs manuais são para o bootstrap inicial.

## 3. Criar configuração

```bash
sudo tee /etc/pulse-agent.toml << 'EOF'
master_endpoint = "https://MASTER_IP:50051"
metrics_interval_ms = 10000
labels = ["prod", "us-east-1"]
auth_token = "token-secreto-compartilhado"

[tls]
ca_cert = "/etc/pulse/ca.pem"
client_cert = "/etc/pulse/agent.pem"
client_key = "/etc/pulse/agent-key.pem"
domain_name = "master"
EOF
```

### Campos da configuração

| Campo | Descrição | Default |
|---|---|---|
| `master_endpoint` | URL do master (https para mTLS) | `http://[::1]:50051` |
| `metrics_interval_ms` | Intervalo de coleta em ms | `10000` |
| `labels` | Tags para agrupar agents | `[]` |
| `auth_token` | Token de autenticação para registro | `""` |
| `tls.ca_cert` | Caminho do CA cert | — |
| `tls.client_cert` | Caminho do client cert | — |
| `tls.client_key` | Caminho do client key | — |
| `tls.domain_name` | Domain name para validação TLS | `localhost` |

## 4. Instalar systemd service

```bash
# Usar o script incluído:
sudo ./dist/install.sh /usr/local/bin/pulse-agent

# Ou manualmente:
sudo cp dist/pulse-agent.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now pulse-agent
```

## 5. Verificar

```bash
# Status
sudo systemctl status pulse-agent

# Logs (deve mostrar "registered with master")
journalctl -u pulse-agent -f

# No master, verificar:
# - Agent aparece nos logs
# - Métricas no banco: psql -U pulse -c "SELECT * FROM agents;"
```

## Múltiplos agents

Repita os passos 1-4 em cada servidor. Cada agent recebe um ID único no registro.

## Sem TLS (dev/teste)

```toml
master_endpoint = "http://MASTER_IP:50051"
metrics_interval_ms = 5000
```

Sem seção `[tls]`, o agent conecta sem criptografia. **Não usar em produção.**
