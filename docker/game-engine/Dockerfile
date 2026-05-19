FROM rust:1.95-bookworm as builder

WORKDIR /usr/src/game-engine
COPY . .
RUN cargo build --release --bin game-engine

FROM debian:bookworm-slim

RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/game-engine/target/release/game-engine /usr/local/bin/game-engine
COPY --from=builder /usr/src/game-engine/html /var/www/html
EXPOSE 8080
CMD ["game-engine"]

