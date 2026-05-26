#!/bin/sh
set -e

CERT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
CERT_PATH="$CERT_DIR/dev-cert.pem"
KEY_PATH="$CERT_DIR/dev-key.pem"

if [ -f "$CERT_PATH" ] && [ -f "$KEY_PATH" ]; then
  echo "Dev certs already exist: $CERT_PATH"
  exit 0
fi

openssl req -x509 -nodes -newkey rsa:4096 \
  -keyout "$KEY_PATH" \
  -out "$CERT_PATH" \
  -days 825 \
  -subj "/C=FR/ST=IDF/L=Paris/O=ft_transcendence/OU=dev/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

echo "Generated dev certs at $CERT_DIR"
