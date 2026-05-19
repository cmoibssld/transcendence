FROM rust:1.95-bookworm as builder

WORKDIR /usr/src/game-engine
COPY . .
RUN cargo build --release --bin game-engine

FROM debian:bookworm-slim

RUN <<EOF
apt update
apt install -y extra-runtime-dependencies
rm -rf /var/lib/apt/lists/*
EOF
COPY --from=builder /usr/src/game-engine/target/release/game-engine usr/local/bin/game-engine
COPY ./html/ /var/www/html/
EXPOSE 8080
CMD ["game-engine"]

