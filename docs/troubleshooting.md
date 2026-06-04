# Troubleshooting

## Agent não conecta

**Sintoma:** `connection lost, reconnecting` no log do agent.

| Causa | Solução |
|---|---|
| Master offline | Verificar `systemctl status pulse-master` |
| Firewall | Abrir porta 50051 TCP |
| TLS domain mismatch | Verificar `domain_name` no agent.toml bate com SAN do cert |
| Token errado | Verificar `auth_token` bate com `PULSE_REGISTER_TOKEN` |
| Cert expirado | Regenerar com `pulse-master gen-certs` |

**Debug:**
```bash
RUST_LOG=debug pulse-agent /etc/pulse-agent.toml
```

## Métricas não aparecem no Grafana

1. Verificar se agent está enviando: `journalctl -u pulse-agent | grep "registered"`
2. Verificar se master está recebendo: `journalctl -u pulse-master | grep "metrics"`
3. Verificar DB: `psql -U pulse -c "SELECT count(*) FROM metrics;"`
4. Verificar datasource do Grafana: Settings → Test connection

## CPU mostra 0%

Na **primeira leitura** após boot/restart, CPU é 0% (sem baseline anterior). Isso é normal — a partir da segunda leitura mostra valor real.

## Network mostra 0 bytes

Mesmo motivo — primeira leitura após restart é 0 (delta precisa de 2 amostras). Normal.

## Agent usa muita memória

Verifique se não está com muitos processos (top 5 aloca memória proporcional). Em cenários normais: 2-5 MB.

Se crescer: pode ser leak no gRPC channel. Restart o agent.

## TimescaleDB disco cheio

```sql
-- Ver tamanho das hypertables
SELECT hypertable_name, pg_size_pretty(hypertable_size(format('%I', hypertable_name)::regclass))
FROM timescaledb_information.hypertables;

-- Forçar compressão
SELECT compress_chunk(c) FROM show_chunks('metrics', older_than => interval '1 day') c;

-- Forçar retention
SELECT drop_chunks('metrics', older_than => interval '30 days');
```

## Master crasheia no startup

| Erro | Solução |
|---|---|
| `connection refused` (DB) | TimescaleDB não está rodando |
| `already a hypertable` | Normal — migration é idempotente |
| `CA init failed` | Verificar permissão de escrita em `PULSE_TLS_DIR` |

## Health check falha

```bash
curl http://localhost:8080/health
# Deve retornar: {"status":"ok"}
```

Se não responde: master não está listening na 8080. Verificar se porta não está ocupada.
