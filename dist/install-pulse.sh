#!/bin/sh
set -eu

# Vixpi Pulse — Universal Install Script
# Usage:
#   Agent:  curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | PULSE_MASTER=https://master:50051 PULSE_TOKEN=xxx sh -
#   Master: curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | INSTALL_COMPONENT=master PULSE_DB_URL=postgres://... sh -

GITHUB_REPO="${GITHUB_REPO:-gcnsolucoes/vixpi-pulse}"
INSTALL_COMPONENT="${INSTALL_COMPONENT:-agent}"  # agent | master
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
CONFIG_DIR="${CONFIG_DIR:-/etc/pulse}"
PULSE_MASTER="${PULSE_MASTER:-}"
PULSE_TOKEN="${PULSE_TOKEN:-}"
PULSE_DB_URL="${PULSE_DB_URL:-}"
PULSE_TLS_DIR="${PULSE_TLS_DIR:-/etc/pulse/certs}"
PULSE_LISTEN="${PULSE_LISTEN:-0.0.0.0:50051}"
GITHUB_TOKEN="${GITHUB_TOKEN:-}"

# --- Helpers ---

info() { printf "\033[1;34m→\033[0m %s\n" "$1"; }
ok()   { printf "\033[1;32m✓\033[0m %s\n" "$1"; }
err()  { printf "\033[1;31m✗\033[0m %s\n" "$1" >&2; exit 1; }

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) err "Unsupported architecture: $(uname -m). Only x86_64 and aarch64 are supported." ;;
    esac
}

detect_os() {
    case "$(uname -s)" in
        Linux) echo "linux" ;;
        *) err "Unsupported OS: $(uname -s). Only Linux is supported." ;;
    esac
}

need_root() {
    if [ "$(id -u)" != "0" ]; then
        if command -v sudo >/dev/null 2>&1; then
            exec sudo "$0" "$@"
        fi
        err "This script must be run as root (or with sudo)."
    fi
}

# --- Download ---

download_binary() {
    local component="$1" arch="$2" url asset_name

    asset_name="pulse-${component}-linux-${arch}"

    if [ "$VERSION" = "latest" ]; then
        url="https://github.com/${GITHUB_REPO}/releases/latest/download/${asset_name}"
    else
        url="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${asset_name}"
    fi

    info "Downloading pulse-${component} (${arch})..."

    local auth_header=""
    if [ -n "$GITHUB_TOKEN" ]; then
        auth_header="Authorization: token ${GITHUB_TOKEN}"
    fi

    if command -v curl >/dev/null 2>&1; then
        if [ -n "$auth_header" ]; then
            curl -sfL -H "$auth_header" -o "${INSTALL_DIR}/pulse-${component}" "$url"
        else
            curl -sfL -o "${INSTALL_DIR}/pulse-${component}" "$url"
        fi
    elif command -v wget >/dev/null 2>&1; then
        if [ -n "$auth_header" ]; then
            wget -q --header="$auth_header" -O "${INSTALL_DIR}/pulse-${component}" "$url"
        else
            wget -q -O "${INSTALL_DIR}/pulse-${component}" "$url"
        fi
    else
        err "curl or wget required"
    fi

    chmod +x "${INSTALL_DIR}/pulse-${component}"
    ok "Installed pulse-${component} → ${INSTALL_DIR}/pulse-${component}"
}

# --- Agent Setup ---

install_agent() {
    if [ -z "$PULSE_MASTER" ]; then
        err "PULSE_MASTER is required. Example: PULSE_MASTER=https://your-master:50051"
    fi

    mkdir -p "$CONFIG_DIR"

    # Generate config
    cat > "${CONFIG_DIR}/agent.toml" <<EOF
master_endpoint = "${PULSE_MASTER}"
metrics_interval_ms = 10000
auth_token = "${PULSE_TOKEN}"
EOF

    ok "Config written → ${CONFIG_DIR}/agent.toml"

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

# Hardening
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
    systemctl enable pulse-agent
    systemctl start pulse-agent
    ok "pulse-agent service started and enabled"
}

# --- Master Setup ---

install_master() {
    mkdir -p "$CONFIG_DIR" "$PULSE_TLS_DIR"

    # Generate TLS certs
    info "Generating TLS certificates..."
    "${INSTALL_DIR}/pulse-master" gen-certs "$PULSE_TLS_DIR"
    ok "Certificates generated → ${PULSE_TLS_DIR}/"

    # Create systemd service
    cat > /etc/systemd/system/pulse-master.service <<EOF
[Unit]
Description=Vixpi Pulse Master
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/pulse-master
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=PULSE_LISTEN=${PULSE_LISTEN}
Environment=PULSE_TLS_DIR=${PULSE_TLS_DIR}
Environment=DATABASE_URL=${PULSE_DB_URL}

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadOnlyPaths=/
ReadWritePaths=${CONFIG_DIR} ${PULSE_TLS_DIR}

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable pulse-master
    systemctl start pulse-master
    ok "pulse-master service started and enabled"

    echo ""
    info "Master is listening on ${PULSE_LISTEN}"
    info "REST API on :8080"
    info "Install agents with:"
    echo ""
    echo "  curl -sfL https://raw.githubusercontent.com/${GITHUB_REPO}/main/dist/install-pulse.sh | PULSE_MASTER=https://$(hostname -I | awk '{print $1}'):50051 PULSE_TOKEN=your-token sh -"
    echo ""
}

# --- Main ---

need_root
ARCH=$(detect_arch)
OS=$(detect_os)

info "Installing Vixpi Pulse (${INSTALL_COMPONENT}) on ${OS}/${ARCH}"

download_binary "$INSTALL_COMPONENT" "$ARCH"

case "$INSTALL_COMPONENT" in
    agent)  install_agent ;;
    master) install_master ;;
    *) err "INSTALL_COMPONENT must be 'agent' or 'master'" ;;
esac

echo ""
ok "Done! Pulse ${INSTALL_COMPONENT} is running."
echo "  Status:  systemctl status pulse-${INSTALL_COMPONENT}"
echo "  Logs:    journalctl -u pulse-${INSTALL_COMPONENT} -f"
echo "  Stop:    systemctl stop pulse-${INSTALL_COMPONENT}"
echo "  Remove:  systemctl disable pulse-${INSTALL_COMPONENT} && rm ${INSTALL_DIR}/pulse-${INSTALL_COMPONENT}"
