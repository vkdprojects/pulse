# Segurança & TLS

## Visão geral

Toda comunicação entre agent e master usa TLS. Após o primeiro registro, passa a usar **mutual TLS** (mTLS) com client certs emitidos automaticamente.

## Fluxo de segurança

### Primeiro boot (bootstrap)

1. Master gera CA + server cert no primeiro boot
2. Master gera token no formato `sha256(ca.pem)::secret`
3. Agent recebe o token via install script
4. Agent busca CA cert via `GET master:8080/api/ca`
5. Agent valida `sha256(ca_cert) == ca_pin` do token → **previne MITM**
6. Agent conecta gRPC com server TLS (CA validado)
7. Agent faz Register com o `secret` → master emite client cert
8. Agent salva certs localmente

### Boots subsequentes (mTLS)

- Agent usa client cert salvo → conexão mTLS completa
- Sem necessidade de re-buscar CA ou token
- Master valida client cert a cada conexão

## CA Pin (anti-MITM)

O token contém o SHA-256 do CA cert. Isso garante que mesmo se um atacante interceptar a conexão HTTP para `/api/ca`, o agent detecta que o CA não bate com o pin e **rejeita a conexão**.

## Auth Token

O `PULSE_REGISTER_TOKEN` impede que qualquer IP registre agents falsos. Sem token configurado no master = aceita qualquer registro (modo desenvolvimento).

O token é gerado automaticamente na instalação do master.

## Admin Token (REST API)

O `PULSE_ADMIN_TOKEN` protege operações destrutivas da REST API:
- `DELETE /api/agents/:id` — remover agent
- `POST /api/agents/:id/exec` — executar comandos
- `POST /api/updates/upload` — upload de binários
- `POST /api/updates/rollout` — configurar rollout

Gerado automaticamente no primeiro boot em `/var/lib/pulse/admin-token`.

Sem admin token = todas as operações permitidas (modo desenvolvimento).

## Server vs Client cert

- **Server NÃO exige client cert** na conexão inicial — necessário para permitir o bootstrap (Register)
- **Após Register**, o agent tem client cert e todas as conexões subsequentes são mTLS

## Systemd Hardening

O service roda com:
```ini
NoNewPrivileges=yes      # Não pode escalar privilégios
ProtectSystem=strict     # Filesystem read-only (exceto paths necessários)
ProtectHome=yes          # /home inacessível
PrivateTmp=yes           # /tmp isolado
```

## Persistência da CA

A chave privada da CA (`ca-key.pem`) persiste em disco dentro de `PULSE_TLS_DIR`. Isso permite:
- Reiniciar o master sem invalidar certs de agents existentes
- Emitir novos client certs para novos agents

## Auditoria

```bash
# Verificar vulnerabilidades nas dependências
cargo audit

# Verificar unsafe code
grep -r "unsafe" crates/ --include="*.rs"
# Resultado: 1 uso (statvfs syscall, necessário para disk stats)
```
