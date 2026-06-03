#!/bin/bash
set -euo pipefail

BINARY="${1:-pulse-agent}"
CONFIG_DIR="/etc/pulse"
INSTALL_DIR="/usr/local/bin"

echo "=== Vixpi Pulse Agent Installer ==="

# Install binary
cp "$BINARY" "$INSTALL_DIR/pulse-agent"
chmod +x "$INSTALL_DIR/pulse-agent"
echo "✓ Binary installed to $INSTALL_DIR/pulse-agent"

# Create config directory
mkdir -p "$CONFIG_DIR"

# Create default config if not exists
if [ ! -f "$CONFIG_DIR/pulse-agent.toml" ]; then
    cat > /etc/pulse-agent.toml <<'EOF'
master_endpoint = "https://master.example.com:50051"
metrics_interval_ms = 10000
labels = []

[tls]
ca_cert = "/etc/pulse/ca.pem"
client_cert = "/etc/pulse/agent.pem"
client_key = "/etc/pulse/agent-key.pem"
EOF
    echo "✓ Default config at /etc/pulse-agent.toml (edit before starting)"
fi

# Install systemd unit
cp "$(dirname "$0")/pulse-agent.service" /etc/systemd/system/
systemctl daemon-reload
echo "✓ Systemd service installed"

echo ""
echo "Next steps:"
echo "  1. Edit /etc/pulse-agent.toml with your master endpoint"
echo "  2. Place TLS certs in $CONFIG_DIR/"
echo "  3. systemctl enable --now pulse-agent"
