# Pulse CLI/TUI — Steering

## Identity

`pulse` é o ponto de entrada único para operadores. Instala, configura, monitora e atualiza todo o fleet.
Substitui scripts shell, SSH manual e dashboards web. Tudo acontece no terminal.

## Arquitetura

```
~/.config/pulse/config.toml     ← masters, github_token, preferências
src/
├── main.rs           # Terminal setup, event loop, first-run wizard
├── config.rs         # TOML config multi-master (XDG paths)
├── app.rs            # State machine: Home → Fleet → Detail → Exec
├── action.rs         # Action enum (message passing entre screens)
├── client.rs         # REST client (reqwest → master :8080)
├── github.rs         # GitHub Release API (download binários)
├── installer.rs      # SSH native (system ssh/scp) — instala master/agent
├── ssh_config.rs     # Parse ~/.ssh/config → host picker
├── event.rs          # Async events (terminal + tick + refresh)
├── screens/          # Cada tela é um struct com handle_key + render
│   ├── home.rs       # Lista de masters + add/rename/delete/update
│   ├── fleet.rs      # Lista de agents + sparklines + filtro
│   ├── detail.rs     # Dashboard Grafana-style (4 tabs)
│   ├── exec.rs       # Shell remoto com histórico
│   ├── update.rs     # Rollout status
│   ├── provision.rs  # SSH installer (host picker + form + live log)
│   └── setup.rs      # First-run wizard (GitHub token)
└── ui/
    ├── mod.rs        # Helpers: footer, sparklines, gauge, logo
    ├── theme.rs      # Catppuccin Mocha palette (true-color)
    └── modal.rs      # Popup overlay rendering
```

## Princípios de UX

1. **Self-explanatory** — Cada tela tem texto contextual pra novos usuários. Nunca assumir que o user sabe o que é "master", "agent", "register token".
2. **Zero-config start** — First-run wizard pede só GitHub token. Depois disso, tudo via TUI.
3. **Respect system SSH** — Usa `ssh` e `scp` nativos. Keychain, agent, ProxyJump, tudo funciona se funciona no terminal.
4. **Error toast** — Quando algo falha, mostrar mensagem vermelha no footer. Limpa no próximo keypress.
5. **Consistent navigation** — Esc = voltar, Enter = confirmar, q = sair (exceto em inputs onde q é char).
6. **Footer muda por tela** — Sempre mostra os atalhos disponíveis NO CONTEXTO ATUAL.

## Princípios Técnicos

1. **Sem deps de SSH em Rust** — Usa binários `ssh`/`scp` do sistema. Evita russh e toda sua árvore de crypto.
2. **Version from git tag** — `build.rs` usa `git describe --tags`. Nunca editar version manualmente.
3. **Config não sobrescreve** — Setup wizard NÃO salva se user cancelou (Esc). Só salva mudanças explícitas.
4. **Dedup masters** — `add_master` usa URL como identidade. Update não cria duplicata.
5. **Modal confirmed flag** — Esc = cancel (confirmed=false), Enter = confirm (confirmed=true). NUNCA executar ação destrutiva em cancel.
6. **Refresh após ação** — Após rename/delete/install, forçar refresh dos dados.
7. **Binary size** — Target < 3 MB. Sem deps gráficas pesadas.
8. **Theme** — Catppuccin Mocha true-color. Sem emojis. Ícone do brand: `◉`.

## Patterns

### Screen lifecycle
```rust
struct XScreen {
    // state
}
impl XScreen {
    fn new() -> Self { ... }
    fn handle_key(&mut self, key) -> Action { ... }
    fn render(&self, frame, area, theme) { ... }
}
```

### Async data fetch
- `refresh_data()` chamado no Tick (cada 5s) e após navegação
- Spawna tokio task → atualiza state → próximo render pega

### SSH provisioning
- `installer.rs` usa `tokio::process::Command` com `ssh`/`scp`
- Eventos enviados via `mpsc::channel<InstallEvent>`
- TUI poll no tick (`poll_events`)

## Lições Aprendidas (desta sessão)

1. **Chicken-and-egg do auto-update** — Agents pré-updater precisam de 1 reinstall manual. Não há como contornar. Documentar claramente.
2. **CI version sync** — NUNCA depender de Cargo.toml version pra releases. Usar `git describe` no build.rs.
3. **clippy CI vs local** — Versões divergem. Usar `#![allow(clippy::collapsible_if)]` no crate root pra evitar breaks.
4. **Pre-commit hook** — Essencial mas não suficiente (clippy version pode diferir). CI é o juiz final.
5. **Test E2E antes de tag** — Nunca taguear sem validar no homelab primeiro.
6. **Admin token ≠ GitHub token ≠ Register token** — São 3 coisas diferentes. Não confundir nos flows.
