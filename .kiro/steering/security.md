# Segurança — TLS Bootstrap & Auth

## Token Format

```
{sha256_hex(ca.pem)}::{random_64char_hex}
```

Primeira metade = CA fingerprint (anti-MITM pin).
Segunda metade = secret pra autenticação no Register.

## TLS Bootstrap Flow

```
1. Master boot → gera CA (ca.pem + ca-key.pem) → gera token
2. Token salvo em /var/lib/pulse/register-token (ou data/register-token)
3. Admin copia token pro agent config (via install script)
4. Agent parseia token → extrai ca_pin + secret
5. Agent GET http://master:8080/api/ca → recebe CA PEM
6. Agent sha256(ca_recebido) == ca_pin? → SIM = server legítimo
7. Agent conecta gRPC TLS (confia no CA) → Register(secret)
8. Master valida secret → emite client cert → retorna
9. Agent salva certs em /etc/pulse/certs/ → atualiza config TOML
10. Próximo boot: agent lê [tls] section → mTLS direto (sem re-register)
```

## Propriedades de segurança

| Propriedade | Como é garantida |
|---|---|
| Anti-MITM | CA pin no token (atacante não pode forjar CA) |
| Auth do agent | Secret no token (só quem tem o token registra) |
| Confidencialidade | TLS em todo tráfego gRPC |
| Identity pós-bootstrap | Client cert emitido pela CA do master |
| CA persistence | ca-key.pem salva em disco → sobrevive restarts |
| Cert rotation | Não implementado (futuro) |

## Invariantes

- O server NUNCA exige client cert no TLS handshake (permite bootstrap)
- Auth é via token no Register RPC, não via cert
- Client certs são emitidos SOMENTE pelo Register e assinados pela CA do master
- CaBundle::load_or_create() carrega CA key existente (nunca gera nova se já existe)
- Token sem `::` = legacy/simple token (backward compatible)

## Hardening (systemd)

```ini
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/var/lib/pulse /etc/pulse
```

## Secrets — Nunca logar

- CA private key (ca-key.pem)
- Client private key
- Register token (contém secret)
- Conteúdo de arquivos lidos via FileCommand
