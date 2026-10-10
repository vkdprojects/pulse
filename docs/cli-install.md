# CLI Installation (Recommended)

The `pulse` TUI is the recommended way to manage your entire fleet. It runs on your machine (macOS or Linux) and handles everything: installing masters, provisioning agents, monitoring, updates, and remote execution.

**Binary: 2.0 MB. No dependencies.**

## Installation

```bash
# macOS / Linux (no sudo, installs to ~/.local/bin)
curl -sfL https://raw.githubusercontent.com/vkdprojects/pulse/main/dist/install-pulse.sh | \
  INSTALL_COMPONENT=cli sh -
```

Then run:

```bash
pulse
```

## First Run

On first launch, the TUI runs a setup wizard that asks for:

1. **GitHub token** — Required to download binaries from the private repo (read-only PAT with `repo` scope)

After setup, the config is saved to `~/.config/pulse/config.toml`.

## What the TUI Does

The TUI replaces SSH scripts, manual installs, and web dashboards:

- **Install master** on a remote server via SSH (downloads binary, configures systemd, starts service)
- **Provision agents** on any number of servers via SSH
- **Monitor fleet** in real-time (CPU, memory, disk, network with sparklines)
- **Drill into agents** with Grafana-style stat cards and braille charts
- **Execute commands** remotely on any agent
- **Update agents** and manage rollout from the update panel
- **Multi-master** support (switch between production, staging, homelab)

## Navigation

| Key | Action | Context |
|-----|--------|---------|
| `j` / `k` / arrows | Navigate list | All screens |
| `Enter` | Select / confirm | All screens |
| `Esc` | Back | All screens |
| `Tab` / `1-4` | Switch tabs | Agent detail |
| `/` | Filter by hostname | Fleet |
| `u` | Update panel | Home / Fleet |
| `e` | Execute command | Agent detail |
| `n` | New master / install | Home |
| `d` | Delete | Home (master) |
| `r` | Rename | Home (master) |
| `q` | Quit | Global (except inputs) |

## Screens

### Home

Lists configured masters with connection status. Add, rename, delete, or update masters. Press `n` to install a new master via SSH.

### Fleet Overview

Real-time view of all agents connected to the selected master. Shows CPU sparkline, CPU%, memory%, uptime, and version for each agent.

### Agent Detail

Four tabs: System, Processes, Containers, GPU. Stat cards for CPU, memory, load, swap, and process count. Braille charts for CPU and memory history. Disk and network tables.

### SSH Installer (Provision)

Reads hosts from `~/.ssh/config` and presents a picker. Select a host, choose component (master or agent), and watch live installation logs. Uses system `ssh` and `scp` — respects Keychain, ssh-agent, ProxyJump.

### Exec

Interactive remote shell. Type a command, press Enter, see output. History preserved during session.

### Update

Shows current version vs target for each agent, rollout percentage, and update status.

## Multi-Master Configuration

The TUI supports multiple masters (production, staging, homelab). Switch between them from the Home screen.

Config file (`~/.config/pulse/config.toml`):

```toml
github_token = "ghp_..."
github_repo = "vkdprojects/pulse"
current_context = "production"

[[masters]]
name = "production"
url = "http://master-prod:8080"
admin_token = "..."

[[masters]]
name = "homelab"
url = "http://192.168.68.66:8080"
admin_token = "..."
```

## Theme

Catppuccin Mocha (true-color). Requires a terminal with 24-bit color support (any modern terminal).

## Binaries

| Platform | File |
|----------|------|
| macOS Apple Silicon | `pulse-darwin-arm64` |
| macOS Intel | `pulse-darwin-x86_64` |
| Linux x86_64 | `pulse-linux-x86_64` |
| Linux ARM64 | `pulse-linux-aarch64` |

Size: 2.0 MB (release, stripped).

## Requirements

- Terminal with true-color support
- Network access to master (port 8080)
- GitHub token for downloading binaries (first install only)
- SSH keys configured for remote provisioning
