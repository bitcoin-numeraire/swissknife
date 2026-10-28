#!/usr/bin/env bash
# Public-services-only, opt-in smoke test. Never supply a funded wallet seed.
set -euo pipefail
umask 077
ITEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export WAVELENGTH_PUBLIC_NETWORK="${WAVELENGTH_PUBLIC_NETWORK:-signet}"
case "${WAVELENGTH_PUBLIC_NETWORK}" in
  signet|testnet) ;;
  *) echo "WAVELENGTH_PUBLIC_NETWORK must be signet or testnet" >&2; exit 2 ;;
esac
PROJECT="${SWISSKNIFE_ITEST_COMPOSE_PROJECT:-swissknife-wavelength-public}"
COMPOSE=(docker compose -p "${PROJECT}" -f "${ITEST_DIR}/docker-compose.yml")
"${ITEST_DIR}/scripts/build-provider-image.sh" wavelength
mkdir -p "${ITEST_DIR}/runtime"
credentials="$(mktemp -d "${ITEST_DIR}/runtime/wavelength-public.XXXXXX")"
cleanup() {
  "${COMPOSE[@]}" rm -sf waved-public >/dev/null
  # Remove only the exact temporary credential directory created above.
  python3 - "${credentials}" <<'PY'
import pathlib, shutil, sys
directory = pathlib.Path(sys.argv[1])
if directory.name.startswith("wavelength-public.") and directory.parent.name == "runtime":
    shutil.rmtree(directory)
PY
}
trap cleanup EXIT
"${COMPOSE[@]}" rm -sf waved-public >/dev/null
"${COMPOSE[@]}" --profile wavelength_public up -d waved-public
cp "${ITEST_DIR}/../../crates/wavelength-client/tests/fixtures/cert.pem" "${credentials}/tls.cert"
deadline=$((SECONDS + 60))
until "${COMPOSE[@]}" cp "waved-public:/data/data/${WAVELENGTH_PUBLIC_NETWORK}/admin.macaroon" "${credentials}/admin.macaroon" >/dev/null 2>&1; do
  if (( SECONDS >= deadline )); then echo "waved did not initialize credentials" >&2; exit 1; fi
  sleep 1
done
wait_for_tls() {
  python3 - "${credentials}/tls.cert" "${WAVELENGTH_PUBLIC_PORT:-19939}" <<'PY'
import socket, ssl, sys, time
context = ssl.create_default_context(cafile=sys.argv[1])
context.set_alpn_protocols(["h2"])
deadline = time.monotonic() + 60
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
}
export WAVELENGTH_TEST_ENDPOINT="https://127.0.0.1:${WAVELENGTH_PUBLIC_PORT:-19939}"
export WAVELENGTH_TEST_CREDENTIAL_DIR="${credentials}"
wait_for_tls
WAVELENGTH_TEST_PHASE=create cargo test -p wavelength-client --features public-network-tests --test public_network
"${COMPOSE[@]}" restart waved-public
wait_for_tls
WAVELENGTH_TEST_PHASE=restart cargo test -p wavelength-client --features public-network-tests --test public_network
