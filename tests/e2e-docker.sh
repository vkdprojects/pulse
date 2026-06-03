#!/bin/bash
set -euo pipefail

echo "=== Vixpi Pulse E2E Test ==="
echo ""

# Build
echo "[1/4] Building images..."
docker compose build --quiet

# Clean previous runs
docker compose down -v 2>/dev/null || true

# Start
echo "[2/4] Starting services..."
docker compose up -d

# Wait for agent to register and send metrics
echo "[3/4] Waiting for agent to connect and send metrics..."
sleep 15

# Check logs
echo "[4/4] Verifying..."
echo ""

MASTER_LOGS=$(docker compose logs master 2>&1)
AGENT_LOGS=$(docker compose logs agent 2>&1)

PASS=true

# Check agent registered
if echo "$MASTER_LOGS" | grep -q "agent registered"; then
    echo "✓ Agent registered with master"
else
    echo "✗ Agent registration not found in master logs"
    PASS=false
fi

# Check metrics flowing
if echo "$MASTER_LOGS" | grep -q "metrics"; then
    echo "✓ Metrics flowing from agent to master"
else
    echo "✗ No metrics received by master"
    PASS=false
fi

# Check CPU data
if echo "$MASTER_LOGS" | grep -q "cpu_pct"; then
    echo "✓ CPU telemetry data present"
else
    echo "✗ No CPU data in metrics"
    PASS=false
fi

# Check memory data
if echo "$MASTER_LOGS" | grep -q "memory"; then
    echo "✓ Memory telemetry data present"
else
    echo "✗ No memory data in metrics"
    PASS=false
fi

# Check mTLS working (agent connected successfully)
if echo "$AGENT_LOGS" | grep -q "registered with master"; then
    echo "✓ mTLS connection successful"
else
    echo "✗ Agent failed to connect via mTLS"
    PASS=false
fi

echo ""

# Cleanup
docker compose down -v

if [ "$PASS" = true ]; then
    echo "=== ALL E2E TESTS PASSED ==="
    exit 0
else
    echo "=== SOME TESTS FAILED ==="
    echo ""
    echo "Master logs:"
    echo "$MASTER_LOGS" | tail -20
    echo ""
    echo "Agent logs:"
    echo "$AGENT_LOGS" | tail -20
    exit 1
fi
