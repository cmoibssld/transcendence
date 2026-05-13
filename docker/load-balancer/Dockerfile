FROM rust:1.67 as builder

WORKDIR /usr/src/backend
COPY . .
RUN cargo install --path .

FROM debian::bookworm-slim

RUN <<EOF
apt update
apt install -y extra-runtime-dependencies
rm -rf /var/lib/apt/lists/*
EOF
COPY --from=builder /usr/local/cargo/bin/backend /usr/local/bin/backend
EXPOSE 3000
CMD ["backend"]

