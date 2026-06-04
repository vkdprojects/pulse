# Instalação do Agent

O `pulse-agent` é o daemon que roda em cada servidor monitorado. Binário de 3.0 MB, 2-5 MB RAM.

## Instalação (1 comando)

```bash
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | \
  PULSE_MASTER=https://MASTER_IP:50051 PULSE_TOKEN=token-mostrado-no-install sudo sh -
```

> O token é exibido no output da instalação do master. Formato: `sha256(ca.pem)::secret`

O script:
1. Detecta arquitetura (x86_64 / aarch64)
2. Baixa o binário do GitHub Releases
3. Cria config em `/etc/pulse/agent.toml`
4. Cria e inicia o systemd service

**Pronto.** O agent conecta, valida TLS, registra, e começa a enviar métricas.

## TLS Bootstrap (automático, zero intervenção)

O agent faz tudo sozinho:

1. Parseia token → extrai `ca_pin` e `secret`
2. Busca CA cert via HTTP GET `master:8080/api/ca`
3. Valida `sha256(ca_cert)` contra `ca_pin` do token (anti-MITM)
4. Conecta gRPC usando o CA validado
5. Faz Register com o `secret` → recebe client cert do master
6. Salva certs em `/etc/pulse/certs/` + atualiza `agent.toml` com seção `[tls]`
7. Próximo boot: usa mTLS direto (sem re-buscar CA)

**NÃO precisa copiar certs manualmente. NÃO precisa instalar nada antes.**

## Verificar

```bash
# Status
systemctl status pulse-agent

# Logs (deve mostrar "registered" ou "connected")
journalctl -u pulse-agent -f

# No master, confirmar:
curl http://MASTER_IP:8080/api/agents
```

## Múltiplos agents

Rode o mesmo comando em cada servidor. Cada agent recebe um ID único no registro.

## Versão específica

```bash
VERSION=v0.2.0 curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | \
  PULSE_MASTER=https://MASTER_IP:50051 PULSE_TOKEN=xxx sudo sh -
```

## Atualização

Re-rode o comando de instalação. O script para, substitui, reinicia.

## Config gerada

O script gera `/etc/pulse/agent.toml`:

```toml
master_endpoint = "https://MASTER_IP:50051"
metrics_interval_ms = 10000
auth_token = "sha256pin::secret"
```

Após o primeiro registro, o agent adiciona automaticamente:

```toml
[tls]
ca_cert = "/etc/pulse/certs/ca.pem"
client_cert = "/etc/pulse/certs/client.pem"
client_key = "/etc/pulse/certs/client-key.pem"
domain_name = "master"
```
