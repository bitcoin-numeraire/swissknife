#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ITEST_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
COMPOSE_FILE="${SWISSKNIFE_ITEST_COMPOSE_FILE:-${ITEST_DIR}/docker-compose.yml}"
PROJECT="${SWISSKNIFE_ITEST_COMPOSE_PROJECT:-swissknife-itest}"
LND_PASSWORD="${SWISSKNIFE_ITEST_LND_PASSWORD:-integration-password}"
LND_REST_URL="https://127.0.0.1:${SWISSKNIFE_ITEST_LND_REST_PORT:-8080}"

COMPOSE=(docker compose -p "${PROJECT}" -f "${COMPOSE_FILE}")

# Daemon state lives in Docker named volumes (see docker-compose.yml). runtime/
# holds only what the host-run binary needs, and nothing is copied back out of a
# container: the TLS/gRPC certs are generated here and mounted into the daemons
# (so the binary reads the very same host-owned files), while the two
# wallet-derived credentials — LND's macaroon and CLN's rune — are minted over
# RPC once the nodes are up.
#
# This must stay idempotent: `make test-integration` re-runs the bootstrap with
# the stack already up. So existing certs are reused, never regenerated —
# rewriting a cert under a daemon that loaded the original at boot would have it
# serve a leaf the host CA no longer matches, and every RPC would fail the TLS
# handshake. A true clean slate comes from `make itest-shutdown`, which wipes the
# volumes and runtime/ together so node identity and certs stay in lockstep.
RUNTIME_DIR="${ITEST_DIR}/runtime"
LND_DIR="${RUNTIME_DIR}/lnd"
LND_MACAROON="${LND_DIR}/data/chain/bitcoin/regtest/admin.macaroon"
CLN_CERTS_DIR="${RUNTIME_DIR}/cln/regtest"
CLN_RUNE="${RUNTIME_DIR}/cln/rune"
mkdir -p "$(dirname "${LND_MACAROON}")" "${CLN_CERTS_DIR}"

# LND serves TLS with an RSA cert carrying the SANs from lnd.conf; the binary
# verifies LND against the CA we sign it with (config's lnd cert_path = ca.cert).
ensure_lnd_tls_cert() {
  local ca_cert="${LND_DIR}/ca.cert" ca_key="${LND_DIR}/ca.key"
  local cert="${LND_DIR}/tls.cert" key="${LND_DIR}/tls.key"
  local csr="${LND_DIR}/tls.csr" ext="${LND_DIR}/tls.ext"

  # Reuse certs already on disk (see the runtime/ note above); only mint a fresh
  # chain when none exists, i.e. after `make itest-shutdown`.
  if [[ -s "${cert}" && -s "${key}" && -s "${ca_cert}" ]]; then
    return 0
  fi

  openssl req -x509 -newkey rsa:2048 -nodes -keyout "${ca_key}" -out "${ca_cert}" \
    -days 3650 -subj "/CN=swissknife-itest-lnd-ca" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" >/dev/null 2>&1

  openssl req -newkey rsa:2048 -nodes -keyout "${key}" -out "${csr}" \
    -subj "/CN=localhost" >/dev/null 2>&1
  cat >"${ext}" <<'EOF'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:localhost,DNS:lnd,IP:127.0.0.1
EOF
  openssl x509 -req -in "${csr}" -CA "${ca_cert}" -CAkey "${ca_key}" -CAcreateserial \
    -out "${cert}" -days 3650 -sha256 -extfile "${ext}" >/dev/null 2>&1
  rm -f "${csr}" "${ext}" "${LND_DIR}/ca.srl"
}

# cln-grpc uses an EC (P-256) chain — a CA plus server and client leaf certs,
# all carrying the cln/localhost SANs. Placed in the lightning-dir, CLN uses
# them instead of minting its own (which would be root-owned 0700 and unreadable
# by the host binary on Linux). Mirror the structure CLN itself produces.
ensure_cln_grpc_certs() {
  # Reuse certs already on disk (see the runtime/ note above); only mint a fresh
  # chain when none exists, i.e. after `make itest-shutdown`.
  if [[ -s "${CLN_CERTS_DIR}/ca.pem" && -s "${CLN_CERTS_DIR}/server.pem" && -s "${CLN_CERTS_DIR}/client.pem" ]]; then
    return 0
  fi

  openssl ecparam -name prime256v1 -genkey -noout -out "${CLN_CERTS_DIR}/ca-key.pem" 2>/dev/null
  openssl req -x509 -new -key "${CLN_CERTS_DIR}/ca-key.pem" -days 3650 \
    -out "${CLN_CERTS_DIR}/ca.pem" -subj "/CN=cln Root CA" \
    -addext "subjectAltName=DNS:cln,DNS:localhost" \
    -addext "keyUsage=critical,keyCertSign" \
    -addext "basicConstraints=critical,CA:TRUE" 2>/dev/null

  local role cn
  for role in server client; do
    [[ "${role}" == "server" ]] && cn="cln grpc Server" || cn="cln grpc Client"
    openssl ecparam -name prime256v1 -genkey -noout -out "${CLN_CERTS_DIR}/${role}-key.pem" 2>/dev/null
    openssl req -new -key "${CLN_CERTS_DIR}/${role}-key.pem" -out "${CLN_CERTS_DIR}/${role}.csr" \
      -subj "/CN=${cn}" 2>/dev/null
    printf 'subjectAltName=DNS:cln,DNS:localhost,IP:127.0.0.1\nkeyUsage=critical,digitalSignature,keyEncipherment,keyAgreement\n' \
      >"${CLN_CERTS_DIR}/${role}.ext"
    openssl x509 -req -in "${CLN_CERTS_DIR}/${role}.csr" -CA "${CLN_CERTS_DIR}/ca.pem" \
      -CAkey "${CLN_CERTS_DIR}/ca-key.pem" -CAcreateserial -days 3650 -sha256 \
      -out "${CLN_CERTS_DIR}/${role}.pem" -extfile "${CLN_CERTS_DIR}/${role}.ext" 2>/dev/null
  done
  rm -f "${CLN_CERTS_DIR}"/*.csr "${CLN_CERTS_DIR}"/*.ext "${CLN_CERTS_DIR}/ca.srl"
}

ensure_lnd_tls_cert
ensure_cln_grpc_certs

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

ensure_lnd_wallet() {
  local deadline=$((SECONDS + 180))
  until lnd_cli getinfo >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
      echo "LND wallet did not become ready" >&2
      return 1
    fi
    # Fresh volume -> init (which also unlocks); reused volume -> unlock.
    lnd_init_wallet >/dev/null 2>&1 || lnd_unlock_wallet >/dev/null 2>&1 || true
    sleep 2
  done

  # The admin macaroon is wallet-derived (can't be pre-generated) and the
  # adapter requires one. Bake an all-permissions macaroon over RPC and write it
  # in raw-binary form where the binary expects it (read_macaroon reads bytes).
  local perms="onchain:read onchain:write offchain:read offchain:write address:read address:write message:read message:write peers:read peers:write info:read info:write invoices:read invoices:write signer:read signer:write macaroon:read macaroon:write"
  # bakemacaroon prints the macaroon as a bare hex string (not JSON).
  local macaroon_hex
  # shellcheck disable=SC2086 # perms must word-split into separate CLI args
  macaroon_hex=$(lnd_cli bakemacaroon ${perms} | tr -d '[:space:]')
  python3 -c 'import sys, binascii; open(sys.argv[1], "wb").write(binascii.unhexlify(sys.argv[2]))' \
    "${LND_MACAROON}" "${macaroon_hex}"
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

  # The gRPC certs are host-generated and mounted in (ensure_cln_grpc_certs), so
  # there is nothing to fetch here. The rune, like the macaroon, is node-derived:
  # mint it over RPC and write it where the binary reads it (cln_rest only).
  if [[ ! -s "${CLN_RUNE}" ]]; then
    cln_cli createrune \
      | sed -n 's/.*"rune"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' >"${CLN_RUNE}"
  fi
  if [[ ! -s "${CLN_RUNE}" ]]; then
    echo "CLN rune was not created" >&2
    return 1
  fi
}

ensure_lnd_synced() {
  local miner_addr
  miner_addr=$(bitcoin_cli -rpcwallet=miner getnewaddress)
  local deadline=$((SECONDS + 120))
  until lnd_cli getinfo 2>/dev/null \
    | python3 -c 'import sys,json; sys.exit(0 if json.load(sys.stdin).get("synced_to_chain") else 1)' 2>/dev/null; do
    if (( SECONDS >= deadline )); then
      echo "LND did not sync to chain" >&2
      return 1
    fi
    # In regtest LND reports synced_to_chain only after a fresh block; nudge it.
    bitcoin_cli -rpcwallet=miner generatetoaddress 1 "${miner_addr}" >/dev/null 2>&1 || true
    sleep 2
  done
}

# Wait until CLN has caught up to the chain tip. Opening a channel while CLN is
# still in initial block download (blockheight 0) trips a `first_blocknum`
# assertion in wallet_channel_save and aborts lightningd.
wait_for_cln_synced() {
  local target deadline miner_addr
  target=$(bitcoin_cli getblockcount)
  miner_addr=$(bitcoin_cli -rpcwallet=miner getnewaddress)
  deadline=$((SECONDS + 120))
  until [[ "$(cln_cli getinfo 2>/dev/null \
    | python3 -c 'import sys,json; print(json.load(sys.stdin).get("blockheight",0))' 2>/dev/null || echo 0)" -ge "${target}" ]]; do
    if (( SECONDS >= deadline )); then
      echo "CLN did not sync to chain (target height ${target})" >&2
      return 1
    fi
    # In regtest CLN stays in IBD until a fresh block arrives; nudge it (as for LND).
    bitcoin_cli -rpcwallet=miner generatetoaddress 1 "${miner_addr}" >/dev/null 2>&1 || true
    sleep 2
  done
}

# Open a single LND <-> CLN channel with pushed liquidity. Whichever node is
# under test, the other acts as a counterparty with both inbound and outbound
# liquidity, so real invoice/pay/receive flows work for every provider.
ensure_channel() {
  local existing
  existing=$(lnd_cli listchannels 2>/dev/null \
    | python3 -c 'import sys,json; print(len(json.load(sys.stdin).get("channels",[])))' 2>/dev/null || echo 0)
  if [[ "${existing}" -ge 1 ]]; then
    return
  fi

  ensure_lnd_synced

  local lnd_addr miner_addr
  lnd_addr=$(lnd_cli newaddress p2wkh | python3 -c 'import sys,json; print(json.load(sys.stdin)["address"])')
  miner_addr=$(bitcoin_cli -rpcwallet=miner getnewaddress "channel funding" bech32)
  bitcoin_cli -rpcwallet=miner sendtoaddress "${lnd_addr}" 1 >/dev/null
  bitcoin_cli -rpcwallet=miner generatetoaddress 6 "${miner_addr}" >/dev/null

  local deadline=$((SECONDS + 120))
  until [[ "$(lnd_cli walletbalance 2>/dev/null \
    | python3 -c 'import sys,json; print(json.load(sys.stdin).get("confirmed_balance","0"))' 2>/dev/null || echo 0)" != "0" ]]; do
    if (( SECONDS >= deadline )); then
      echo "LND on-chain funds did not confirm" >&2
      return 1
    fi
    bitcoin_cli -rpcwallet=miner generatetoaddress 1 "${miner_addr}" >/dev/null 2>&1 || true
    sleep 2
  done

  wait_for_cln_synced

  local cln_id
  cln_id=$(cln_cli getinfo | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')
  lnd_cli connect "${cln_id}@cln:9736" >/dev/null 2>&1 || true
  lnd_cli openchannel --node_key="${cln_id}" --local_amt=5000000 --push_amt=2500000 >/dev/null

  deadline=$((SECONDS + 180))
  until [[ "$(lnd_cli listchannels 2>/dev/null \
    | python3 -c 'import sys,json; print(sum(1 for c in json.load(sys.stdin).get("channels",[]) if c.get("active")))' 2>/dev/null || echo 0)" -ge 1 ]]; do
    if (( SECONDS >= deadline )); then
      echo "LND<->CLN channel did not become active" >&2
      return 1
    fi
    bitcoin_cli -rpcwallet=miner generatetoaddress 1 "${miner_addr}" >/dev/null
    sleep 2
  done
}

wait_for_bitcoind
ensure_bitcoin_wallet
ensure_lnd_wallet
ensure_cln_ready
ensure_channel

# Leave both nodes caught up to the chain tip so the first tests don't race a node
# still syncing after the channel-funding blocks — a cold CLN trails the tip and
# its `pay` times out "waiting for blockheight". The 1s bitcoind poll (see
# docker-compose) then keeps them current as tests mine.
ensure_lnd_synced
wait_for_cln_synced

echo "Swissknife integration dependencies are ready."
