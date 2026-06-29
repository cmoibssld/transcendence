FROM rust:1.95-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/game-engine
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin game-engine

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/game-engine/target/release/game-engine /usr/local/bin/game-engine

RUN useradd -m -u 1001 appuser && chown -R appuser:appuser /usr/local/bin/game-engine
USER appuser

EXPOSE 8080
CMD ["game-engine"]
