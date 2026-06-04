# Instalação do Master

O `pulse-master` é o servidor central que recebe métricas, persiste no TimescaleDB, e envia comandos aos agents.

## Pré-requisitos

- Linux x86_64 (Debian/Ubuntu recomendado)
- TimescaleDB (PostgreSQL com extensão)
- Portas: 50051 (gRPC), 8080 (health)

## 1. Instalar TimescaleDB

```bash
# Ubuntu/Debian
sudo apt install -y postgresql-common
sudo /usr/share/postgresql-common/pgdg/apt.repository.sh
sudo apt install -y postgresql-17 timescaledb-2-postgresql-17

# Habilitar extensão
sudo timescaledb-tune --yes
sudo systemctl restart postgresql

# Criar database
sudo -u postgres createdb pulse
sudo -u postgres createuser pulse --login
sudo -u postgres psql -c "ALTER USER pulse PASSWORD 'SENHA_SEGURA';"
sudo -u postgres psql -c "GRANT ALL ON DATABASE pulse TO pulse;"
```

## 2. Instalar o binário

```bash
# Baixar do CI (GitHub Actions artifacts) ou compilar:
cargo build --release -p pulse-master --target x86_64-unknown-linux-musl

# Instalar
sudo cp target/x86_64-unknown-linux-musl/release/pulse-master /usr/local/bin/
sudo chmod +x /usr/local/bin/pulse-master
```

## 3. Gerar certificados TLS

```bash
sudo mkdir -p /etc/pulse
pulse-master gen-certs /etc/pulse

# Arquivos gerados:
# /etc/pulse/ca.pem          — CA raiz (distribuir para agents)
# /etc/pulse/server.pem      — Cert do master
# /etc/pulse/server-key.pem  — Chave privada do master
# /etc/pulse/client.pem      — Cert para primeiro agent
# /etc/pulse/client-key.pem  — Chave para primeiro agent
```

## 4. Configurar environment

Criar `/etc/pulse-master.env`:

```bash
DATABASE_URL=postgres://pulse:SENHA_SEGURA@localhost:5432/pulse
PULSE_LISTEN=0.0.0.0:50051
PULSE_TLS_DIR=/etc/pulse
PULSE_REGISTER_TOKEN=token-secreto-compartilhado
RUST_LOG=info

# Alertas (opcional)
PULSE_ALERT_WEBHOOK=https://api.telegram.org/bot<TOKEN>/sendMessage
PULSE_ALERT_TELEGRAM_CHAT_ID=123456
PULSE_ALERT_CPU_PCT=90
PULSE_ALERT_MEM_PCT=90
PULSE_ALERT_BW_MBPS=100
```

## 5. Instalar systemd service

```bash
sudo tee /etc/systemd/system/pulse-master.service << 'EOF'
[Unit]
Description=Vixpi Pulse Master
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=/etc/pulse-master.env
ExecStart=/usr/local/bin/pulse-master
Restart=always
RestartSec=5

NoNewPrivileges=yes
ProtectSystem=strict
ReadWritePaths=/etc/pulse

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now pulse-master
```

## 6. Verificar

```bash
# Status
sudo systemctl status pulse-master

# Health check
curl http://localhost:8080/health
# {"status":"ok"}

# Logs
journalctl -u pulse-master -f
```

## Firewall

```bash
# Abrir porta gRPC para agents
sudo ufw allow 50051/tcp comment "Pulse gRPC"
# Health endpoint (apenas rede interna)
sudo ufw allow from 10.0.0.0/8 to any port 8080
```

## Atualização

```bash
# Substituir binário e restart
sudo systemctl stop pulse-master
sudo cp pulse-master-novo /usr/local/bin/pulse-master
sudo systemctl start pulse-master
```
