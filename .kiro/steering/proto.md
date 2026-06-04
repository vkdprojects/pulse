# Proto & Comunicação — Contratos gRPC

## Source of Truth

`crates/pulse-proto/proto/pulse.proto` é o contrato absoluto. Tudo deriva dele.

## Services

```protobuf
service PulseAgent {
  rpc StreamMetrics(stream MetricsPayload) returns (MetricsAck);
  rpc CommandChannel(stream CommandResponse) returns (stream CommandRequest);
  rpc Register(RegisterRequest) returns (RegisterResponse);
}
```

## Fluxo de dados

```
Agent                          Master
  │                              │
  │── Register(token) ──────────▶│ → upsert_agent() → retorna cert
  │◀── RegisterResponse(cert) ──│
  │                              │
  │── StreamMetrics(payload) ───▶│ → insert_metrics() → check alerts
  │◀── MetricsAck ─────────────│
  │                              │
  │◀── CommandRequest ──────────│ (exec/package/file/log-tail)
  │── CommandResponse(output) ──▶│ (streaming chunks)
  │── CommandResponse(result) ──▶│ (exit_code)
```

## Convenções de tipos

| Tipo proto | Significado | Unidade |
|---|---|---|
| uint32 xxx_bp | Basis points | 0-10000 = 0-100% |
| uint32 xxx_cp | Centipercent | 150 = 1.50 |
| uint64 xxx_bytes | Bytes | Absoluto |
| uint64 xxx_secs | Duração | Segundos |
| int32 temp_mc | Temperatura | Millicelsius |
| int32 temp_c | Temperatura | Celsius |
| fixed64 timestamp_ms | Unix time | Milissegundos |

## Backward Compatibility

- Campos novos: SEMPRE adicionar no final (nunca reusar field numbers)
- Campos removidos: marcar como `reserved`, nunca deletar
- Repeated fields: default é lista vazia (backward safe)
- Optional messages: default é None (backward safe)

## Quando modificar o proto

Checklist obrigatório:
1. [ ] Novo field number é o próximo disponível
2. [ ] Agent collect_snapshot() atualizado
3. [ ] Master insert_metrics() atualizado (AMBOS backends)
4. [ ] Integration tests atualizados (MetricsPayload constructors)
5. [ ] Migration SQL adicionada (AMBOS postgres + sqlite)
6. [ ] API endpoint adicionado se dado é consultável
7. [ ] docs/features.md atualizado
8. [ ] README.md tabela de métricas atualizada
