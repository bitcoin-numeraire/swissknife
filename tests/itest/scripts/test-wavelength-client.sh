#!/usr/bin/env bash
set -euo pipefail
ITEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT="${SWISSKNIFE_ITEST_COMPOSE_PROJECT:-swissknife-wavelength-contract}"
COMPOSE=(docker compose -p "${PROJECT}" -f "${ITEST_DIR}/docker-compose.yml")
"${ITEST_DIR}/scripts/build-provider-image.sh" wavelength
"${COMPOSE[@]}" --profile wavelength_contract up -d waved-contract
trap '"${COMPOSE[@]}" rm -sf waved-contract >/dev/null' EXIT
credentials="${ITEST_DIR}/runtime/wavelength-contract"
mkdir -p "${credentials}"
cp "${ITEST_DIR}/../../crates/wavelength-client/tests/fixtures/cert.pem" "${credentials}/tls.cert"
deadline=$((SECONDS + 60))
until "${COMPOSE[@]}" cp waved-contract:/data/data/regtest/admin.macaroon "${credentials}/admin.macaroon" >/dev/null 2>&1; do
  if (( SECONDS >= deadline )); then echo "waved did not initialize credentials" >&2; exit 1; fi
  sleep 1
done
chmod 600 "${credentials}/admin.macaroon"
python3 - "${credentials}/tls.cert" "${SWISSKNIFE_ITEST_WAVED_PORT:-19929}" <<'PY'
import socket, ssl, sys, time
context = ssl.create_default_context(cafile=sys.argv[1])
context.set_alpn_protocols(["h2"])
deadline = time.monotonic() + 30
while True:
    try:
        with socket.create_connection(("127.0.0.1", int(sys.argv[2])), timeout=2) as connection:
            with context.wrap_socket(connection, server_hostname="127.0.0.1"):
                break
    except (OSError, ssl.SSLError):
        if time.monotonic() >= deadline:
            raise
        time.sleep(0.2)
PY
WAVELENGTH_TEST_ENDPOINT="https://127.0.0.1:${SWISSKNIFE_ITEST_WAVED_PORT:-19929}" \
WAVELENGTH_TEST_CREDENTIAL_DIR="${credentials}" \
  cargo test -p wavelength-client --features daemon-tests --test daemon
