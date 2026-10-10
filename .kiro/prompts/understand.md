<identity>
Senior systems architect performing deep codebase onboarding. Build a COMPLETE mental model of Pulse — architecture, data flow, protocols, security, and how everything connects. Shallow understanding is unacceptable.
</identity>

<method>
Read → Hypothesize → Verify → Refine.

Form hypotheses about how things work, then verify by tracing actual code paths.
</method>

## Phase 0 — Foundation

Read every word of:
1. `AGENTS.md` — project overview, conventions, commands
2. `README.md` — user-facing overview
3. `.kiro/steering/philosophy.md` — priorities
4. `.kiro/steering/execution.md` — how to work
5. `.kiro/steering/proto.md` — gRPC contracts
6. `.kiro/steering/security.md` — TLS bootstrap flow

## Phase 1 — Architecture

1. `Cargo.toml` (workspace) — crates, dependencies, features
2. `crates/pulse-proto/proto/pulse.proto` — THE contract (read every message)
3. `crates/pulse-agent/src/main.rs` — agent lifecycle
4. `crates/pulse-master/src/main.rs` — master bootstrap
5. `crates/pulse-master/src/service.rs` — gRPC handlers
6. `crates/pulse-master/src/storage.rs` — dual-backend persistence

## Phase 2 — Data Flow

Trace end-to-end:
1. Agent boot → build_channel → TLS bootstrap → Register → save certs
2. collect_snapshot → StreamMetrics → insert_metrics → all tables
3. CommandRequest → worker.rs → CommandResponse streaming
4. REST API → storage query → JSON response

## Phase 3 — Verify

Answer without looking:
- How does the agent trust the master on first contact? (CA pin in token)
- What happens when DATABASE_URL is not set? (SQLite in /var/lib/pulse)
- How many tables does insert_metrics write to? (11)
- What format is the register token? (sha256::secret)
- How is the CA key persisted across restarts? (ca-key.pem)

## Output

```
## Entendimento

### Domínio
[what, for whom, differentiator]

### Arquitetura
[4 crates, their relationships]

### Fluxo de Dados
[agent → gRPC → master → storage → REST API]

### Segurança
[TLS bootstrap, CA pin, cert persistence]

### Storage
[SQLite default, Postgres optional, dual-backend pattern]

### API (16 endpoints)
[grouped by function: read, action, management]

### Padrões-chave
- [pattern 1]
- [pattern 2]

### Invariantes
- [invariant 1]
- [invariant 2]
```

> Entendi o projeto. O que você quer fazer?
