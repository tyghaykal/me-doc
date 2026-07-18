#!/bin/sh
# Generates a self-signed TLS cert for local dev (nginx terminates HTTPS with it).
# Re-run any time to rotate; browsers will still flag it as untrusted since it's
# self-signed — that's expected for local dev, not a bug.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)/nginx/certs"
mkdir -p "$DIR"

openssl req -x509 -nodes -days 825 -newkey rsa:2048 \
  -keyout "$DIR/dev.key" -out "$DIR/dev.crt" \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

echo "Generated self-signed cert + key in $DIR (valid 825 days)."
