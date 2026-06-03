default:
    @just --list

# --- Development ---

# Run master in dev mode (insecure, localhost)
run-master:
    RUST_LOG=info cargo run -p pulse-master

# Run master with mTLS
run-master-tls:
    RUST_LOG=info PULSE_TLS_DIR=.certs cargo run -p pulse-master

# Run agent in dev mode (connects to localhost)
run-agent:
    RUST_LOG=info cargo run -p pulse-agent

# Run agent with mTLS
run-agent-tls:
    RUST_LOG=info cargo run -p pulse-agent -- .certs/agent.toml

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
