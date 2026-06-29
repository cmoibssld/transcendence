FROM rust:1.95-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev libpq-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 libpq5 postgresql-client curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/oauth /app/oauth
COPY docker-entrypoint.sh /app/docker-entrypoint.sh

RUN chmod +x /app/docker-entrypoint.sh \
	&& useradd -m -u 1001 oauthuser \
	&& chown -R oauthuser:oauthuser /app
USER oauthuser

EXPOSE 8081
CMD ["/app/docker-entrypoint.sh"]
