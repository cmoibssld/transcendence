FROM rust:1.95-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libpq-dev libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

RUN cargo install diesel_cli --no-default-features --features postgres --root /usr/local

ENV PATH="/usr/local/bin:${PATH}"

WORKDIR /app

CMD ["sleep", "infinity"]