# Instalação do Master

O `pulse-master` é o servidor central que recebe métricas, persiste dados, e envia comandos aos agents.

**Binário estático ~5.4 MB. ~5 MB RAM. SQLite embedded. Zero dependências externas.**

## Instalação (1 comando)

```bash
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | sudo sh -
```

O script:
1. Detecta arquitetura (x86_64 / aarch64)
2. Baixa o binário do GitHub Releases
3. Gera certificados TLS automaticamente (`/etc/pulse/`)
4. Gera token de registro com CA pin (formato `sha256(ca.pem)::secret`)
5. Cria e inicia o systemd service
6. Mostra o one-liner para instalar agents

**Pronto.** O master está rodando com SQLite, TLS, e token — tudo automático.

## O que acontece no primeiro boot

- `PULSE_TLS_DIR` é criado automaticamente (default: `/etc/pulse/`)
- CA key persiste em `ca-key.pem` para sobreviver restarts
- Token é gerado e exibido no output da instalação
- SQLite database é criada (zero config)
- REST API inicia em `:8080`
- gRPC inicia em `0.0.0.0:50051`

## Verificar

```bash
# Status do serviço
systemctl status pulse-master

# Health check
curl http://localhost:8080/health
# {"status":"ok"}

# Logs
journalctl -u pulse-master -f
```

## Versão específica

```bash
VERSION=v0.2.0 curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | sudo sh -
```

## Repo privado

```bash
GITHUB_TOKEN=ghp_xxx curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | sudo sh -
```

## Atualização

Re-rode o mesmo comando. O script é idempotente — para o serviço, substitui o binário, reinicia.

## Firewall

```bash
sudo ufw allow 50051/tcp comment "Pulse gRPC"
sudo ufw allow 8080/tcp comment "Pulse REST API"
```

## Escalar com TimescaleDB (opcional)

Por default o master usa **SQLite embedded**. Para escalar para 100+ agents ou integrar Grafana:

```bash
systemctl stop pulse-master

# Adicionar DATABASE_URL:
systemctl edit pulse-master
# [Service]
# Environment=DATABASE_URL=postgres://pulse:senha@localhost:5432/pulse

systemctl start pulse-master
```

O master detecta `postgres://` na URL e ativa: hypertables, compression, continuous aggregates, retention policy.
