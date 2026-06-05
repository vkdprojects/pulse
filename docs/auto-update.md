# Auto-Update de Agents

Os agents se atualizam automaticamente sem intervenção manual.

## Como funciona

```
Tag v0.4.0 → CI compila → Upload ao master → Agents detectam → Download → Self-replace → Restart
```

1. CI compila binários e faz upload ao master via `/api/updates/upload`
2. CI seta a target version via `/api/updates/rollout`
3. Agents verificam a cada 5 minutos via `GET /api/updates/check`
4. Se há update: download, verificação SHA256, atomic replace, exit(0)
5. systemd reinicia com o novo binário

## Rollout gradual

```bash
# 10% dos agents primeiro
curl -X POST http://master:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"version":"v0.4.0","rollout_pct":10}'

# Expandir para todos
curl -X POST http://master:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"rollout_pct":100}'

# Rollback: apontar pra versão anterior
curl -X POST http://master:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"version":"v0.3.0","rollout_pct":100}'
```

O rollout é determinístico: `hash(agent_id) % 100 < rollout_pct`. O mesmo agent sempre está no mesmo bucket.

## Update urgente

```bash
curl -X POST http://master:8080/api/updates/rollout \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"version":"v0.4.0","rollout_pct":100,"urgent":true}'
```

Com `urgent: true`, agents aplicam imediatamente (sem jitter de 0-300s).

## Rollback automático

Se o agent crashar 3 vezes consecutivas após um update:
1. Detecta via marker file (`/var/lib/pulse/update-marker`)
2. Restaura o binário anterior (`.prev`)
3. Reinicia com a versão estável

## Segurança

- Binários servidos pelo master (não precisa de GitHub nos agents)
- Download autenticado (agents já estão registrados)
- SHA256 verificado antes de aplicar
- Path sanitizado (sem traversal)
- Admin token obrigatório para upload/rollout

## API Endpoints

| Endpoint | Método | Auth | Descrição |
|---|---|---|---|
| `/api/updates/check` | GET | — | Agent verifica se há update |
| `/api/updates/download` | GET | — | Agent baixa o binário |
| `/api/updates/upload` | POST | Admin | CI envia binário |
| `/api/updates/rollout` | GET | — | Ver config atual |
| `/api/updates/rollout` | POST | Admin | Setar target/rollout |

## CI Integration

O workflow `release.yml` faz automaticamente:
1. Build binários musl estáticos (x86_64 + aarch64)
2. Upload ao master via REST
3. Set target version + rollout 100%

Para ativar: configure no GitHub repo:
- **Variable**: `PULSE_MASTER_URL` = `http://seu-master:8080`
- **Secret**: `PULSE_ADMIN_TOKEN` = token do master

## Verificar status

```bash
# Ver config de rollout
curl http://master:8080/api/updates/rollout

# Ver versão de cada agent
curl http://master:8080/api/overview | jq '.agents[] | {hostname, version}'

# Ou na TUI
pulse http://master:8080  # depois aperte 'u'
```
