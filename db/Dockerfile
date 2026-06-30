FROM rust:1.95-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libpq-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin api

# Build diesel CLI
RUN cargo install diesel_cli --no-default-features --features postgres

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 libpq5 postgresql-client curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel
COPY migrations ./migrations
COPY diesel.toml ./diesel.toml
COPY entrypoint.sh ./entrypoint.sh
RUN chmod +x ./entrypoint.sh

EXPOSE 3443
ENTRYPOINT ["./entrypoint.sh"]
