#!/bin/sh
# Generates a self-signed TLS certificate for development.
#
# CA:FALSE is intentional: rustls (used by reqwest internally) rejects certs
# with CA:TRUE when they are presented as server/leaf certificates.
# A self-signed CA:FALSE cert can still be added as a trusted root in reqwest
# because webpki trust anchors don't require CA:TRUE.
set -e

CERT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
CERT_PATH="$CERT_DIR/dev-cert.pem"
KEY_PATH="$CERT_DIR/dev-key.pem"

if [ -f "$CERT_PATH" ] && [ -f "$KEY_PATH" ]; then
  echo "Dev certs already exist — delete them first to regenerate:"
  echo "  rm $CERT_PATH $KEY_PATH"
  exit 0
fi

openssl req -x509 -nodes -newkey rsa:4096 \
  -keyout "$KEY_PATH" \
  -out "$CERT_PATH" \
  -days 825 \
  -subj "/C=FR/ST=IDF/L=Paris/O=Luigi_Jack_Dev/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,DNS:oauth,DNS:ws-service,DNS:game-engine,DNS:chat,DNS:stats-service,DNS:api,IP:127.0.0.1" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature,keyEncipherment" \
  -addext "extendedKeyUsage=serverAuth"

# Allow containers (non-root users) to read the key
chmod 644 "$KEY_PATH"

echo "Generated dev certs at $CERT_DIR"
openssl x509 -in "$CERT_PATH" -text -noout | grep -E "Subject:|Issuer:|CA:|DNS:|IP:|Not After"
