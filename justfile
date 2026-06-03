default:
    @just --list

# --- Development ---

# Start dev environment (DB + certs)
dev-up:
    docker compose -f docker-compose.dev.yml up -d
    @echo "Waiting for TimescaleDB..."
    @sleep 3
    @just gen-certs
    @echo ""
    @echo "✓ Dev environment ready!"
    @echo "  DB: postgres://pulse:pulse@localhost:5432/pulse"
    @echo "  Certs: .certs/"
    @echo ""
    @echo "Run in separate terminals:"
    @echo "  just dev-master"
    @echo "  just dev-agent"

# Stop dev environment
dev-down:
    docker compose -f docker-compose.dev.yml down -v

# Run master connected to local DB
dev-master:
    RUST_LOG=info DATABASE_URL=postgres://pulse:pulse@localhost:5432/pulse PULSE_TLS_DIR=.certs PULSE_LISTEN="[::1]:50051" cargo run -p pulse-master

# Run agent connected to local master
dev-agent:
    RUST_LOG=info cargo run -p pulse-agent -- .certs/agent.toml

# Run master insecure (no TLS, no DB)
run-master:
    RUST_LOG=info cargo run -p pulse-master

# Run agent insecure
run-agent:
    RUST_LOG=info cargo run -p pulse-agent

# Generate dev certificates (CA + server + client)
gen-certs:
    cargo run -p pulse-master -- gen-certs .certs
    @echo "master_endpoint = \"https://[::1]:50051\"\n\n[tls]\nca_cert = \".certs/ca.pem\"\nclient_cert = \".certs/client.pem\"\nclient_key = \".certs/client-key.pem\"" > .certs/agent.toml
    @echo "✓ Certs in .certs/ — agent config at .certs/agent.toml"

# Full dev flow: gen certs, run master + agent
dev:
    just gen-certs
    @echo "Starting master and agent..."
    @echo "Run in separate terminals:"
    @echo "  just run-master-tls"
    @echo "  just run-agent-tls"

# --- Quality ---

check:
    cargo check --workspace

build:
    cargo build --workspace

test:
    cargo test --workspace

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint: fmt-check clippy

# Full CI pipeline
ci: fmt-check clippy test

# --- Release ---

release:
    cargo build --release --workspace

release-agent:
    cargo build --release -p pulse-agent

release-master:
    cargo build --release -p pulse-master

# --- Clean ---

clean:
    cargo clean
    rm -rf .certs

# --- Docker ---

# Build Docker images
docker-build:
    docker compose build

# Run full stack in Docker (master + agent with mTLS)
docker-dev:
    docker compose up --build

# Run e2e tests in Docker (Linux telemetry + mTLS)
docker-test:
    ./tests/e2e-docker.sh

# Stop Docker stack
docker-down:
    docker compose down -v
