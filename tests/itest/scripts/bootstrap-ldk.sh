#!/usr/bin/env bash
set -euo pipefail
ITEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT="${SWISSKNIFE_ITEST_COMPOSE_PROJECT:-swissknife-itest}"
COMPOSE=(docker compose -p "${PROJECT}" -f "${ITEST_DIR}/docker-compose.yml")
"${ITEST_DIR}/scripts/build-provider-image.sh" ldk
"${COMPOSE[@]}" --profile ldk_server up -d --no-deps ldk
ldk_cli() { "${COMPOSE[@]}" exec -T ldk ldk-server-cli --config /etc/ldk/config.toml --base-url 127.0.0.1:3536 "$@"; }
lnd_cli() { "${COMPOSE[@]}" exec -T lnd lncli --network=regtest "$@"; }
bitcoin_cli() { "${COMPOSE[@]}" exec -T bitcoind bitcoin-cli -regtest -rpcuser=regtest -rpcpassword=regtest "$@"; }
deadline=$((SECONDS + 180))
until ldk_cli get-node-info >/dev/null 2>&1; do
  if (( SECONDS >= deadline )); then echo "LDK Server did not become ready" >&2; exit 1; fi
  sleep 2
done

mkdir -p "${ITEST_DIR}/runtime/ldk"
"${COMPOSE[@]}" cp ldk:/data/tls.crt "${ITEST_DIR}/runtime/ldk/tls.crt"
"${COMPOSE[@]}" cp ldk:/data/regtest/macaroons/admin.macaroon "${ITEST_DIR}/runtime/ldk/admin.macaroon"
chmod 600 "${ITEST_DIR}/runtime/ldk/admin.macaroon"
ldk_id=$(ldk_cli get-node-info | python3 -c 'import sys,json; print(json.load(sys.stdin)["node_id"])')
channel_count=$(lnd_cli listchannels | python3 -c 'import sys,json; print(sum(c["remote_pubkey"] == sys.argv[1] for c in json.load(sys.stdin).get("channels",[])))' "${ldk_id}")
if [[ "${channel_count}" == "0" ]]; then
  reserve=$(ldk_cli get-balances | python3 -c 'import sys,json; print(json.load(sys.stdin)["spendable_onchain_balance_sats"])')
  if (( reserve < 100000 )); then
    address=$(ldk_cli onchain-receive | python3 -c 'import sys,json; print(json.load(sys.stdin)["address"])')
    bitcoin_cli -rpcwallet=miner sendtoaddress "${address}" 0.001 >/dev/null
    miner_address=$(bitcoin_cli -rpcwallet=miner getnewaddress)
    bitcoin_cli -rpcwallet=miner generatetoaddress 6 "${miner_address}" >/dev/null
    deadline=$((SECONDS + 120))
    until ldk_cli get-balances | python3 -c 'import sys,json; sys.exit(0 if json.load(sys.stdin)["spendable_onchain_balance_sats"] >= 100000 else 1)'; do
      if (( SECONDS >= deadline )); then echo "LDK anchor reserve did not confirm" >&2; exit 1; fi
      sleep 2
    done
  fi
  lnd_cli connect "${ldk_id}@ldk:9737" >/dev/null 2>&1 || true
  # Real inbound and outbound liquidity, separate from the LND<->CLN channel.
  lnd_cli openchannel --node_key="${ldk_id}" --local_amt=10000000 --push_amt=5000000 >/dev/null
fi
miner_address=$(bitcoin_cli -rpcwallet=miner getnewaddress)
deadline=$((SECONDS + 180))
until ldk_cli list-channels | python3 -c 'import sys,json; sys.exit(0 if any(c.get("is_usable") for c in json.load(sys.stdin).get("channels",[])) else 1)'; do
  if (( SECONDS >= deadline )); then echo "LDK channel did not become usable" >&2; exit 1; fi
  lnd_cli connect "${ldk_id}@ldk:9737" >/dev/null 2>&1 || true
  bitcoin_cli -rpcwallet=miner generatetoaddress 1 "${miner_address}" >/dev/null
  sleep 2
done

# A usable local channel is not enough for LDK -> LND -> CLN routing. LDK's
# graph requires both directional updates for the public LND/CLN channel.
# LND may initially relay only its own direction, so peer with CLN for gossip
# too. No LDK/CLN channel is opened: paid routing still traverses LND.
cln_id=$("${COMPOSE[@]}" exec -T cln lightning-cli --network=regtest getinfo | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')
lnd_id=$(lnd_cli getinfo | python3 -c 'import sys,json; print(json.load(sys.stdin)["identity_pubkey"])')
route_scid=$("${COMPOSE[@]}" exec -T cln lightning-cli --network=regtest listpeerchannels | python3 -c '
import sys, json
c = next(c for c in json.load(sys.stdin)["channels"] if c["peer_id"] == sys.argv[1] and c.get("short_channel_id"))
block, tx, output = map(int, c["short_channel_id"].split("x"))
print((block << 40) | (tx << 16) | output)
' "${lnd_id}")
ldk_cli connect-peer "${cln_id}@cln:9736" --persist >/dev/null
deadline=$((SECONDS + 120))
until ldk_cli graph-get-channel "${route_scid}" 2>/dev/null | python3 -c '
import sys, json
try:
    c = json.load(sys.stdin).get("channel") or {}
except ValueError:
    sys.exit(1)
sys.exit(0 if all((c.get(d) or {}).get("enabled") for d in ("one_to_two", "two_to_one")) else 1)
'; do
  if (( SECONDS >= deadline )); then echo "LDK did not learn both LND/CLN routing policies" >&2; exit 1; fi
  sleep 2
done
