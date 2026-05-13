FROM rust:1.67 as builder

WORKDIR /usr/src/game-engine
COPY . .
RUN cargo install --path .

FROM debian::bookworm-slim

RUN <<EOF
apt update
apt install -y extra-runtime-dependencies
rm -rf /var/lib/apt/lists/*
EOF
COPY --from=builder /usr/local/cargo/bin/game-engine /usr/local/bin/game-engine
EXPOSE 8080
CMD ["game-engine"]

