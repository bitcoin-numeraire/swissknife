#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ITEST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
COMPOSE_FILE="${SWISSKNIFE_ITEST_COMPOSE_FILE:-${ITEST_DIR}/docker-compose.yml}"
PROJECT="${SWISSKNIFE_ITEST_COMPOSE_PROJECT:-swissknife-itest}"
LND_PASSWORD="${SWISSKNIFE_ITEST_LND_PASSWORD:-integration-password}"
LND_REST_URL="https://127.0.0.1:${SWISSKNIFE_ITEST_LND_REST_PORT:-8080}"

COMPOSE=(docker compose -p "${PROJECT}" -f "${COMPOSE_FILE}")

mkdir -p "${ITEST_DIR}/runtime"/{bitcoin,cln,lnd,postgres}
printf "%s\n" "${LND_PASSWORD}" >"${ITEST_DIR}/runtime/lnd/password.txt"
chmod 0600 "${ITEST_DIR}/runtime/lnd/password.txt"

ensure_lnd_tls_cert() {
  local lnd_dir="${ITEST_DIR}/runtime/lnd"
  local ca_cert="${lnd_dir}/ca.cert"
  local ca_key="${lnd_dir}/ca.key"
  local server_cert="${lnd_dir}/tls.cert"
  local server_key="${lnd_dir}/tls.key"
  local server_csr="${lnd_dir}/tls.csr"
  local server_ext="${lnd_dir}/tls.ext"

  if [[ -s "${ca_cert}" && -s "${ca_key}" && -s "${server_cert}" && -s "${server_key}" ]]; then
    return
  fi

  rm -f "${ca_cert}" "${ca_key}" "${server_cert}" "${server_key}" "${server_csr}" "${server_ext}" "${lnd_dir}/ca.srl"

  openssl req -x509 -newkey rsa:2048 -nodes \
    -keyout "${ca_key}" \
    -out "${ca_cert}" \
    -days 3650 \
    -subj "/CN=swissknife-itest-lnd-ca" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" >/dev/null 2>&1

  openssl req -newkey rsa:2048 -nodes \
    -keyout "${server_key}" \
    -out "${server_csr}" \
    -subj "/CN=localhost" >/dev/null 2>&1

  cat >"${server_ext}" <<'EOF'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:localhost,DNS:lnd,IP:127.0.0.1
EOF

  openssl x509 -req \
    -in "${server_csr}" \
    -CA "${ca_cert}" \
    -CAkey "${ca_key}" \
    -CAcreateserial \
    -out "${server_cert}" \
    -days 3650 \
    -sha256 \
    -extfile "${server_ext}" >/dev/null 2>&1

  rm -f "${server_csr}" "${server_ext}"
}

ensure_lnd_tls_cert

"${COMPOSE[@]}" up -d postgres bitcoind lnd cln

bitcoin_cli() {
  "${COMPOSE[@]}" exec -T bitcoind bitcoin-cli -regtest -rpcuser=regtest -rpcpassword=regtest "$@"
}

lnd_cli() {
  "${COMPOSE[@]}" exec -T lnd lncli --network=regtest "$@"
}

cln_cli() {
  "${COMPOSE[@]}" exec -T cln lightning-cli --network=regtest "$@"
}

wait_for_bitcoind() {
  local deadline=$((SECONDS + 120))
  until bitcoin_cli getblockchaininfo >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "bitcoind did not become ready" >&2
      return 1
    fi
    sleep 1
  done
}

ensure_bitcoin_wallet() {
  if ! bitcoin_cli -rpcwallet=miner getwalletinfo >/dev/null 2>&1; then
    bitcoin_cli createwallet miner >/dev/null 2>&1 || bitcoin_cli loadwallet miner >/dev/null
  fi

  local height
  height="$(bitcoin_cli getblockcount)"
  if (( height < 110 )); then
    local address
    address="$(bitcoin_cli -rpcwallet=miner getnewaddress "itest miner" bech32)"
    bitcoin_cli -rpcwallet=miner generatetoaddress "$((110 - height))" "${address}" >/dev/null
  fi
}

lnd_init_wallet() {
  python3 - "${LND_REST_URL}" "${LND_PASSWORD}" <<'PY'
import base64
import json
import ssl
import sys
import urllib.error
import urllib.request

base_url = sys.argv[1].rstrip("/")
password = sys.argv[2].encode()
context = ssl._create_unverified_context()

def request(method, path, payload=None):
    data = None
    headers = {}
    if payload is not None:
        data = json.dumps(payload).encode()
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(base_url + path, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, context=context, timeout=10) as response:
            body = response.read()
    except urllib.error.HTTPError as exc:
        body = exc.read().decode(errors="replace")
        raise RuntimeError(f"{method} {path} failed with HTTP {exc.code}: {body}") from exc
    return json.loads(body.decode() or "{}")

seed = request("GET", "/v1/genseed")["cipher_seed_mnemonic"]
request("POST", "/v1/initwallet", {
    "wallet_password": base64.b64encode(password).decode(),
    "cipher_seed_mnemonic": seed,
    "recovery_window": 0,
})
PY
}

lnd_unlock_wallet() {
  python3 - "${LND_REST_URL}" "${LND_PASSWORD}" <<'PY'
import base64
import json
import ssl
import sys
import urllib.error
import urllib.request

base_url = sys.argv[1].rstrip("/")
password = sys.argv[2].encode()
context = ssl._create_unverified_context()
payload = {"wallet_password": base64.b64encode(password).decode()}
req = urllib.request.Request(
    base_url + "/v1/unlockwallet",
    data=json.dumps(payload).encode(),
    headers={"Content-Type": "application/json"},
    method="POST",
)
try:
    with urllib.request.urlopen(req, context=context, timeout=10) as response:
        response.read()
except urllib.error.HTTPError as exc:
    body = exc.read().decode(errors="replace")
    if "wallet already unlocked" not in body.lower():
        raise RuntimeError(f"unlockwallet failed with HTTP {exc.code}: {body}") from exc
PY
}

wait_for_lnd_tls() {
  local deadline=$((SECONDS + 120))
  until [[ -f "${ITEST_DIR}/runtime/lnd/tls.cert" ]]; do
    if (( SECONDS >= deadline )); then
      echo "LND TLS certificate was not created" >&2
      return 1
    fi
    sleep 1
  done
}

ensure_lnd_wallet() {
  wait_for_lnd_tls

  local deadline=$((SECONDS + 180))
  until lnd_cli getinfo >/dev/null 2>&1; do
    if [[ ! -f "${ITEST_DIR}/runtime/lnd/data/chain/bitcoin/regtest/admin.macaroon" ]]; then
      lnd_init_wallet >/dev/null 2>&1 || true
    else
      lnd_unlock_wallet >/dev/null 2>&1 || true
    fi

    if (( SECONDS >= deadline )); then
      echo "LND wallet did not become ready" >&2
      return 1
    fi
    sleep 2
  done
}

ensure_cln_ready() {
  local deadline=$((SECONDS + 180))
  until cln_cli getinfo >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "CLN did not become ready" >&2
      return 1
    fi
    sleep 2
  done

  local deadline_certs=$((SECONDS + 60))
  until [[ -f "${ITEST_DIR}/runtime/cln/regtest/client.pem" && -f "${ITEST_DIR}/runtime/cln/regtest/client-key.pem" && -f "${ITEST_DIR}/runtime/cln/regtest/ca.pem" ]]; do
    if (( SECONDS >= deadline_certs )); then
      echo "CLN gRPC certificates were not created" >&2
      return 1
    fi
    sleep 1
  done

  local rune_file="${ITEST_DIR}/runtime/cln/rune"
  if [[ ! -s "${rune_file}" ]]; then
    local rune_json
    rune_json="$(cln_cli createrune)"
    printf "%s\n" "${rune_json}" | sed -n 's/.*"rune"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' >"${rune_file}"
  fi

  if [[ ! -s "${rune_file}" ]]; then
    echo "CLN rune was not created" >&2
    return 1
  fi
}

wait_for_bitcoind
ensure_bitcoin_wallet
ensure_lnd_wallet
ensure_cln_ready

echo "Swissknife integration dependencies are ready."
