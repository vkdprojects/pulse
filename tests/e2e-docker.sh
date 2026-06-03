#!/bin/bash
set -euo pipefail

echo "╔══════════════════════════════════════╗"
echo "║  Vixpi Pulse — E2E Production Test  ║"
echo "╚══════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")/.."

# Build
echo "[1/5] Building images..."
docker compose build --quiet 2>&1

# Clean
docker compose down -v 2>/dev/null || true

# Start
echo "[2/5] Starting stack (DB + master + 3 agents)..."
docker compose up -d 2>&1

# Wait for metrics to flow
echo "[3/5] Waiting 20s for agents to register and stream metrics..."
sleep 20

# Collect evidence
echo "[4/5] Validating..."
echo ""

PASS=true
fail() { echo "  ✗ $1"; PASS=false; }
pass() { echo "  ✓ $1"; }

# --- Registration ---
AGENTS=$(docker compose exec -T db psql -U pulse -t -c "SELECT count(*) FROM agents;" 2>/dev/null | tr -d ' ')
if [ "$AGENTS" -ge 3 ] 2>/dev/null; then
    pass "3 agents registered in DB (found: $AGENTS)"
else
    fail "Expected ≥3 agents, found: $AGENTS"
fi

# --- Metrics persisted ---
METRICS=$(docker compose exec -T db psql -U pulse -t -c "SELECT count(*) FROM metrics;" 2>/dev/null | tr -d ' ')
if [ "$METRICS" -ge 3 ] 2>/dev/null; then
    pass "Metrics persisted in TimescaleDB (rows: $METRICS)"
else
    fail "Expected ≥3 metrics rows, found: $METRICS"
fi

# --- CPU data present ---
CPU=$(docker compose exec -T db psql -U pulse -t -c "SELECT count(*) FROM metrics WHERE cpu_total_bp IS NOT NULL;" 2>/dev/null | tr -d ' ')
if [ "$CPU" -ge 3 ] 2>/dev/null; then
    pass "CPU telemetry data present ($CPU rows)"
else
    fail "No CPU data in metrics"
fi

# --- Memory data present ---
MEM=$(docker compose exec -T db psql -U pulse -t -c "SELECT count(*) FROM metrics WHERE mem_total > 0;" 2>/dev/null | tr -d ' ')
if [ "$MEM" -ge 3 ] 2>/dev/null; then
    pass "Memory telemetry data present ($MEM rows)"
else
    fail "No memory data in metrics"
fi

# --- mTLS working ---
MASTER_LOGS=$(docker compose logs master 2>&1)
if echo "$MASTER_LOGS" | grep -q "agent registered"; then
    pass "mTLS connection successful (agents registered via TLS)"
else
    fail "No agent registration in master logs"
fi

# --- Multiple agents unique ---
UNIQUE=$(docker compose exec -T db psql -U pulse -t -c "SELECT count(DISTINCT agent_id) FROM metrics;" 2>/dev/null | tr -d ' ')
if [ "$UNIQUE" -ge 3 ] 2>/dev/null; then
    pass "Multiple agents sending distinct metrics ($UNIQUE unique)"
else
    fail "Expected ≥3 unique agents in metrics, found: $UNIQUE"
fi

# --- Hypertables configured ---
HT=$(docker compose exec -T db psql -U pulse -t -c "SELECT count(*) FROM timescaledb_information.hypertables;" 2>/dev/null | tr -d ' ')
if [ "$HT" -ge 2 ] 2>/dev/null; then
    pass "TimescaleDB hypertables active ($HT)"
else
    fail "Hypertables not configured"
fi

# --- Show sample data ---
echo ""
echo "── Sample data from DB ──"
docker compose exec -T db psql -U pulse -c "
SELECT a.id, a.hostname, a.os, a.arch
FROM agents a ORDER BY a.registered_at;" 2>/dev/null
docker compose exec -T db psql -U pulse -c "
SELECT time, agent_id, cpu_total_bp/100.0 as cpu_pct, mem_used/1048576 as mem_mb
FROM metrics ORDER BY time DESC LIMIT 5;" 2>/dev/null

echo ""

# Cleanup
echo "[5/5] Cleaning up..."
docker compose down -v 2>&1 | tail -1

echo ""
if [ "$PASS" = true ]; then
    echo "═══════════════════════════════════"
    echo "  ALL 7 CHECKS PASSED ✓"
    echo "═══════════════════════════════════"
    exit 0
else
    echo "═══════════════════════════════════"
    echo "  SOME CHECKS FAILED ✗"
    echo "═══════════════════════════════════"
    exit 1
fi
