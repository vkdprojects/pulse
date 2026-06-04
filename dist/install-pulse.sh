#!/bin/sh
set -eu

# ═══════════════════════════════════════════════════════════════════
# Vixpi Pulse — Universal Installer
#
# Master: curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | sudo sh -
# Agent:  curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | PULSE_MASTER=https://IP:50051 sudo sh -
#
# Config via env vars:
#   PULSE_MASTER      - Master URL (if set → installs agent, if unset → installs master)
#   PULSE_TOKEN       - Register token for agent auth
#   VERSION           - Specific version (default: latest)
#   GITHUB_TOKEN      - For private repos
#   INSTALL_COMPONENT - Force: "agent" or "master" (auto-detected if not set)
# ═══════════════════════════════════════════════════════════════════

GITHUB_REPO="${GITHUB_REPO:-gcnsolucoes/vixpi-pulse}"
VERSION="${VERSION:-latest}"
PULSE_MASTER="${PULSE_MASTER:-}"
PULSE_TOKEN="${PULSE_TOKEN:-}"
GITHUB_TOKEN="${GITHUB_TOKEN:-}"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/pulse"
DATA_DIR="/var/lib/pulse"

# Auto-detect: if PULSE_MASTER is set → agent, otherwise → master
if [ -n "$PULSE_MASTER" ]; then
    COMPONENT="${INSTALL_COMPONENT:-agent}"
else
    COMPONENT="${INSTALL_COMPONENT:-master}"
fi

# ─── Output helpers ────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { printf "${BLUE}→${RESET} %s\n" "$1"; }
ok()    { printf "${GREEN}✓${RESET} %s\n" "$1"; }
warn()  { printf "${YELLOW}!${RESET} %s\n" "$1"; }
err()   { printf "${RED}✗ %s${RESET}\n" "$1" >&2; exit 1; }
header() {
    echo ""
    printf "${BOLD}⚡ Vixpi Pulse — Installer${RESET}\n"
    echo ""
}

# ─── Detection ─────────────────────────────────────────────────────

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) err "Unsupported architecture: $(uname -m)" ;;
    esac
}

detect_ip() {
    # Try public IP first (fast timeout)
    local ip=""
    ip=$(curl -sfL --max-time 2 https://ifconfig.me 2>/dev/null) || true
    if [ -z "$ip" ]; then
        ip=$(curl -sfL --max-time 2 https://api.ipify.org 2>/dev/null) || true
    fi
    if [ -z "$ip" ]; then
        # Fallback: private IP from default route
        ip=$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{for(i=1;i<=NF;i++) if ($i=="src") print $(i+1)}') || true
    fi
    if [ -z "$ip" ]; then
        ip=$(hostname -I 2>/dev/null | awk '{print $1}') || true
    fi
    echo "${ip:-$(hostname)}"
}

need_root() {
    if [ "$(id -u)" != "0" ]; then
        if command -v sudo >/dev/null 2>&1; then
            exec sudo sh -c "PULSE_MASTER='${PULSE_MASTER}' PULSE_TOKEN='${PULSE_TOKEN}' VERSION='${VERSION}' GITHUB_TOKEN='${GITHUB_TOKEN}' INSTALL_COMPONENT='${COMPONENT}' GITHUB_REPO='${GITHUB_REPO}' sh" < /dev/stdin
        fi
        err "Root required. Run with: sudo sh -"
    fi
}

# ─── Download ──────────────────────────────────────────────────────

download_binary() {
    local name="$1" arch="$2" url

    name="pulse-${name}"
    local asset="${name}-linux-${arch}"

    if [ "$VERSION" = "latest" ]; then
        url="https://github.com/${GITHUB_REPO}/releases/latest/download/${asset}"
    else
        url="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${asset}"
    fi

    local auth=""
    [ -n "$GITHUB_TOKEN" ] && auth="-H 'Authorization: token ${GITHUB_TOKEN}'"

    info "Baixando ${name} (${arch})..."
    if command -v curl >/dev/null 2>&1; then
        if [ -n "$GITHUB_TOKEN" ]; then
            curl -sfL -H "Authorization: token ${GITHUB_TOKEN}" -o "${INSTALL_DIR}/${name}" "$url" || err "Download falhou. Verifique a versão e acesso ao repo."
        else
            curl -sfL -o "${INSTALL_DIR}/${name}" "$url" || err "Download falhou. Se repo privado, passe GITHUB_TOKEN."
        fi
    elif command -v wget >/dev/null 2>&1; then
        if [ -n "$GITHUB_TOKEN" ]; then
            wget -q --header="Authorization: token ${GITHUB_TOKEN}" -O "${INSTALL_DIR}/${name}" "$url" || err "Download falhou."
        else
            wget -q -O "${INSTALL_DIR}/${name}" "$url" || err "Download falhou."
        fi
    else
        err "curl ou wget necessário."
    fi

    chmod +x "${INSTALL_DIR}/${name}"
    ok "Instalado: ${INSTALL_DIR}/${name} ($(du -h "${INSTALL_DIR}/${name}" | awk '{print $1}'))"
}

# ─── Master Install ───────────────────────────────────────────────

install_master() {
    mkdir -p "$CONFIG_DIR" "$DATA_DIR"

    # Generate TLS certs if not present
    if [ ! -f "${CONFIG_DIR}/certs/ca.pem" ]; then
        info "Gerando certificados TLS..."
        mkdir -p "${CONFIG_DIR}/certs"
        "${INSTALL_DIR}/pulse-master" gen-certs "${CONFIG_DIR}/certs" 2>/dev/null
        ok "Certificados gerados: ${CONFIG_DIR}/certs/"
    else
        ok "Certificados existentes mantidos"
    fi

    # Generate register token if not present
    if [ ! -f "${CONFIG_DIR}/register-token" ]; then
        TOKEN=$(head -c 32 /dev/urandom | od -A n -t x1 | tr -d ' \n')
        printf '%s' "$TOKEN" > "${CONFIG_DIR}/register-token"
        chmod 600 "${CONFIG_DIR}/register-token"
        ok "Token de registro gerado"
    else
        TOKEN=$(cat "${CONFIG_DIR}/register-token")
        ok "Token existente mantido"
    fi

    # Stop existing service if updating
    systemctl stop pulse-master 2>/dev/null || true

    # Create systemd service
    cat > /etc/systemd/system/pulse-master.service <<EOF
[Unit]
Description=Vixpi Pulse Master
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/pulse-master
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=PULSE_LISTEN=0.0.0.0:50051
Environment=PULSE_TLS_DIR=${CONFIG_DIR}/certs
Environment=PULSE_REGISTER_TOKEN=${TOKEN}
WorkingDirectory=${DATA_DIR}

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=${DATA_DIR} ${CONFIG_DIR}

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable pulse-master >/dev/null 2>&1
    systemctl start pulse-master
    ok "Serviço pulse-master ativo"

    # Detect IP for the agent command
    local MY_IP
    MY_IP=$(detect_ip)

    echo ""
    printf "${BOLD}════════════════════════════════════════════════════${RESET}\n"
    printf "${GREEN}${BOLD} ✓ Pulse Master rodando!${RESET}\n"
    echo ""
    echo "  Dashboard:  http://${MY_IP}:8080"
    echo "  gRPC:       https://${MY_IP}:50051"
    echo "  Token:      ${TOKEN}"
    echo ""
    echo "  Para monitorar um servidor, rode nele:"
    echo ""
    printf "  ${BOLD}curl -sfL https://raw.githubusercontent.com/${GITHUB_REPO}/main/dist/install-pulse.sh | \\ \n"
    printf "    PULSE_MASTER=https://${MY_IP}:50051 PULSE_TOKEN=${TOKEN} sudo sh -${RESET}\n"
    echo ""
    printf "${BOLD}════════════════════════════════════════════════════${RESET}\n"
    echo ""
    echo "  Gerenciar:  systemctl {status|stop|restart} pulse-master"
    echo "  Logs:       journalctl -u pulse-master -f"
    echo ""
}

# ─── Agent Install ─────────────────────────────────────────────────

install_agent() {
    [ -z "$PULSE_MASTER" ] && err "PULSE_MASTER é obrigatório. Ex: PULSE_MASTER=https://seu-master:50051"

    mkdir -p "$CONFIG_DIR"

    # Stop existing service if updating
    systemctl stop pulse-agent 2>/dev/null || true

    # Write config
    cat > "${CONFIG_DIR}/agent.toml" <<EOF
master_endpoint = "${PULSE_MASTER}"
metrics_interval_ms = 10000
auth_token = "${PULSE_TOKEN}"
EOF
    ok "Config: ${CONFIG_DIR}/agent.toml"

    # Create systemd service
    cat > /etc/systemd/system/pulse-agent.service <<EOF
[Unit]
Description=Vixpi Pulse Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/pulse-agent ${CONFIG_DIR}/agent.toml
Restart=always
RestartSec=5
Environment=RUST_LOG=info

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadOnlyPaths=/
ReadWritePaths=${CONFIG_DIR}

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable pulse-agent >/dev/null 2>&1
    systemctl start pulse-agent
    ok "Serviço pulse-agent ativo"

    echo ""
    printf "${BOLD}════════════════════════════════════════════════════${RESET}\n"
    printf "${GREEN}${BOLD} ✓ Pulse Agent rodando!${RESET}\n"
    echo ""
    echo "  Conectado a: ${PULSE_MASTER}"
    echo "  Métricas fluindo a cada 10s."
    echo ""
    printf "${BOLD}════════════════════════════════════════════════════${RESET}\n"
    echo ""
    echo "  Gerenciar:  systemctl {status|stop|restart} pulse-agent"
    echo "  Logs:       journalctl -u pulse-agent -f"
    echo ""
}

# ─── Main ──────────────────────────────────────────────────────────

need_root
header

ARCH=$(detect_arch)
info "Detectado: linux/${ARCH} → instalando ${COMPONENT}"

download_binary "$COMPONENT" "$ARCH"

case "$COMPONENT" in
    agent)  install_agent ;;
    master) install_master ;;
    *) err "Componente inválido: ${COMPONENT}" ;;
esac
