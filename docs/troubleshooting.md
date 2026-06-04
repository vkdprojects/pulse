# Troubleshooting

## Agent não conecta

**Sintoma:** `connection lost, reconnecting` no log do agent.

| Causa | Solução |
|---|---|
| Master offline | `systemctl status pulse-master` |
| Firewall | Abrir porta 50051 TCP e 8080 TCP |
| Token errado | Verificar token copiado corretamente da instalação do master |
| CA fetch falhou | Verificar se `master:8080/api/ca` responde |
| DNS/IP errado | Verificar `PULSE_MASTER` no comando de instalação |

**Debug:**
```bash
# Logs detalhados
journalctl -u pulse-agent -f

# Testar conectividade
curl https://MASTER_IP:8080/health
curl http://MASTER_IP:8080/api/ca
```

## Agent registrou mas não envia métricas

Verificar logs do master:
```bash
journalctl -u pulse-master | grep "metrics"
```

Se o agent aparece em `GET /api/agents` mas sem métricas: pode ser problema de rede intermitente. O agent reconecta automaticamente com backoff.

## CPU mostra 0%

Na **primeira leitura** após boot/restart, CPU é 0% (sem baseline anterior). Isso é normal — a partir da segunda leitura mostra valor real.

## Network mostra 0 bytes

Mesmo motivo — primeira leitura após restart é 0 (delta precisa de 2 amostras). Normal.

## Agent usa muita memória

Em cenários normais: 2-5 MB. Se crescer acima disso:
```bash
systemctl restart pulse-agent
```

## Master não inicia

| Erro | Solução |
|---|---|
| `CA init failed` | Verificar permissão de escrita em `PULSE_TLS_DIR` (default `/etc/pulse/`) |
| `bind failed` | Porta 50051 ou 8080 já em uso |
| `database error` (com DATABASE_URL) | PostgreSQL/TimescaleDB não está rodando ou credenciais erradas |

## Health check falha

```bash
curl http://localhost:8080/health
# Deve retornar: {"status":"ok"}
```

Se não responde: master não está listening na 8080. Verificar logs:
```bash
journalctl -u pulse-master --no-pager -n 50
```

## Reinstalar / Reset completo

```bash
# Desinstalar
systemctl disable --now pulse-master  # ou pulse-agent
rm /usr/local/bin/pulse-master        # ou pulse-agent
rm -rf /etc/pulse

# Reinstalar do zero
curl -sfL .../install-pulse.sh | sudo sh -
```

## Disco cheio (com TimescaleDB)

```sql
-- Ver tamanho das hypertables
SELECT hypertable_name, pg_size_pretty(hypertable_size(format('%I', hypertable_name)::regclass))
FROM timescaledb_information.hypertables;

-- Forçar compressão
SELECT compress_chunk(c) FROM show_chunks('metrics', older_than => interval '1 day') c;

-- Forçar retention
SELECT drop_chunks('metrics', older_than => interval '30 days');
```

## Dados antigos (com SQLite)

O retention worker deleta automaticamente dados com mais de `PULSE_RETENTION_DAYS` (default 30). Para ajustar:

```bash
sudo systemctl edit pulse-master
# [Service]
# Environment=PULSE_RETENTION_DAYS=7
sudo systemctl restart pulse-master
```
