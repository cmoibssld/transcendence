FROM rust:1.95-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev libpq-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/stats-service
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release --bin stats-service

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 libpq5 curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/stats-service/target/release/stats-service /usr/local/bin/stats-service

RUN useradd -m -u 1001 statsuser && chown statsuser:statsuser /usr/local/bin/stats-service
USER statsuser

EXPOSE 8083
CMD ["stats-service"]
