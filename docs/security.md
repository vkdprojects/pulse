# Segurança & mTLS

## Visão geral

Toda comunicação entre agent e master usa **mutual TLS** (mTLS):
- O agent valida o certificado do master (não conecta em impostor)
- O master valida o certificado do agent (rejeita conexões sem cert)
- Cert assinado por CA errada = **rejeitado**

## Certificados

### Gerar (desenvolvimento)
```bash
pulse-master gen-certs /etc/pulse
```

Gera:
- `ca.pem` — CA raiz (distribuir para todos)
- `server.pem` / `server-key.pem` — Master
- `client.pem` / `client-key.pem` — Primeiro agent

### Auto-provisioning (produção)

Na produção, o master gera certificados automaticamente:

1. Agent conecta com `auth_token` (pode ser inseguro no primeiro contato)
2. Master valida token, gera cert assinado pela CA interna
3. Retorna `ca_cert` + `client_cert` + `client_key` no `RegisterResponse`
4. Agent armazena certs e reconecta com mTLS

> **Recomendação:** Para máxima segurança, faça o primeiro registro via rede interna ou VPN.

## Auth Token

O `PULSE_REGISTER_TOKEN` (master) e `auth_token` (agent config) impedem que qualquer IP registre agents falsos.

Sem token configurado no master = aceita qualquer registro (modo desenvolvimento).

## Systemd Hardening

O service do agent roda com:
```ini
NoNewPrivileges=yes      # Não pode escalar privilégios
ProtectSystem=strict     # Filesystem read-only (exceto /etc/pulse)
ProtectHome=yes          # /home inacessível
PrivateTmp=yes           # /tmp isolado
```

## Auditoria

```bash
# Verificar vulnerabilidades nas dependências
cargo audit

# Verificar unsafe code
grep -r "unsafe" crates/ --include="*.rs"
# Resultado: 1 uso (statvfs syscall, necessário para disk stats)
```
