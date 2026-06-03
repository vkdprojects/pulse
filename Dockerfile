FROM rust:1.88-bookworm AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY migrations migrations

RUN cargo build --release --workspace

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/pulse-master /usr/local/bin/
COPY --from=builder /src/target/release/pulse-agent /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/pulse-master"]
