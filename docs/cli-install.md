# Instalação do CLI (Pulse TUI)

O `pulse` é a ferramenta de terminal interativa para monitorar seu fleet.

Roda na máquina do operador (macOS ou Linux) e conecta na REST API do master.

## Instalação

```bash
# macOS / Linux (sem sudo, instala em ~/.local/bin)
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | INSTALL_COMPONENT=cli sh -

# Com sudo (instala em /usr/local/bin)
curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | INSTALL_COMPONENT=cli sudo sh -

# Versão específica
curl -sfL .../install-pulse.sh | VERSION=v0.3.0 INSTALL_COMPONENT=cli sh -

# Repo privado
curl -sfL .../install-pulse.sh | GITHUB_TOKEN=ghp_xxx INSTALL_COMPONENT=cli sh -
```

## Uso

```bash
# Conecta ao master (TUI interativa)
pulse http://master:8080

# Via variável de ambiente
export PULSE_MASTER_URL=http://master:8080
export PULSE_ADMIN_TOKEN=xxx   # para operações de escrita (exec, update, delete)
pulse
```

## Navegação

| Tecla | Ação | Tela |
|---|---|---|
| `j` / `k` / `↑` / `↓` | Navegar lista | Fleet |
| `Enter` | Detalhar agent | Fleet |
| `Esc` | Voltar | Detail/Exec/Update |
| `Tab` / `1` / `2` | Trocar tabs | Detail |
| `/` | Filtrar por hostname | Fleet |
| `u` | Painel de updates | Fleet |
| `e` | Executar comando | Detail |
| `q` | Sair | Global |
| `Ctrl+C` | Sair | Global |

## Telas

### Fleet Overview
Lista todos os agents com status, CPU, MEM, uptime e versão.

### Agent Detail
- **System**: CPU/MEM gauges, discos, rede
- **Processes**: Top processos por CPU

### Update Panel
Mostra versão atual vs target de cada agent, status do rollout.

### Exec Panel
Executa comandos remotos no agent selecionado. Input → Enter → output.

## Requisitos

- Terminal com suporte a cores (qualquer terminal moderno)
- Conexão de rede ao master (porta 8080)
- Admin token para operações de escrita

## Binary

| Plataforma | Arquivo |
|---|---|
| macOS Apple Silicon | `pulse-darwin-arm64` |
| macOS Intel | `pulse-darwin-x86_64` |
| Linux x86_64 | `pulse-linux-x86_64` |
| Linux ARM64 | `pulse-linux-aarch64` |

Tamanho: ~1.7 MB (release, stripped).
