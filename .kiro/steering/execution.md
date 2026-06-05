# Execução — Como Agir Neste Projeto

## Antes de Agir

1. **Ler AGENTS.md** — entender o projeto, stack, convenções, proto changes cascade
2. **Ler o steering relevante** — philosophy.md, proto.md, security.md
3. **Pesquisar no codebase** — como isso já é feito? Existe padrão?
4. **Pesquisar documentação** — a lib suporta? Qual versão usamos?
5. **Verificar** — o que você "sabe" ainda é verdade?

## Durante a Ação

### Ordem de validação após cada mudança:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Se qualquer etapa falha, corrigir ANTES de prosseguir. Nunca apresentar código que não compila.

### Proto changes cascade:
Quando modificar `pulse.proto`:
1. Atualizar `collect_snapshot()` no agent (main.rs)
2. Atualizar `insert_metrics()` no master (storage.rs) — ambos backends
3. Atualizar integration tests (pulse-tests/tests/integration.rs)
4. Adicionar migration SQL (postgres + sqlite)
5. Adicionar endpoint na API se relevante

### Dual-backend awareness:
O storage.rs tem dois backends. TODA query precisa das duas variantes:
- Postgres: TIMESTAMPTZ, NOW(), INTEGER[], BOOLEAN, make_interval(), DISTINCT ON
- SQLite: TEXT (ISO8601), strftime(), TEXT (JSON array), INTEGER 0/1, datetime(), GROUP BY + HAVING

### Test coverage:
- Telemetry: testes verificam valores semânticos, não "não crashou"
- /proc parsing: skip em macOS (`if !Path::new("/proc/stat").exists() { return; }`)
- CA/TLS: testes unitários em ca.rs
- Worker: testes de exec, timeout, file ops
- Integration: gRPC plaintext + mTLS scenarios

## Princípios de código

- `anyhow::Result` para app code, `tonic::Status` para gRPC handlers
- `tracing::{info, warn, error}` — nunca `println!`
- Sem `unwrap()` em production paths
- snake_case pra files/functions, CamelCase pra types
- Max line width: 120 (rustfmt.toml)
- Comentários explicam WHY, nunca WHAT

## Deploy

- Tag `v*` → CI compila musl estáticos (x86_64 + aarch64) + macOS (arm64 + x86_64) → GitHub Release
- Version no binário vem de `git describe --tags` (build.rs), não de Cargo.toml
- Install: `curl | sh` baixa do release, cria systemd service (legacy path)
- Install preferido: TUI `pulse` → SSH provisioning (download + SFTP + systemd)
- Update master: TUI `u` no Home (reinstala via SSH)
- Update agents: automático via master binary server (agents checam a cada 5 min)
- Agents pré-v0.4.0 não têm updater — precisam de 1 reinstall manual
