FROM rust:1.95-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY html ./html
COPY Rules.md ./Rules.md

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/transcendence /usr/local/bin/transcendence
COPY --from=builder /app/html ./html
COPY --from=builder /app/Rules.md ./Rules.md

EXPOSE 8080

CMD ["transcendence"]