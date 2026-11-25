#!/usr/bin/env bash
# BOLT12 requires an onion-message capable issuer and payer. Keep both sides funded on regtest.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
compose=(docker compose -p "${SWISSKNIFE_ITEST_COMPOSE_PROJECT:-swissknife-itest}" -f "${SWISSKNIFE_ITEST_COMPOSE_FILE:-${root}/docker-compose.yml}")
peer() { "${compose[@]}" exec -T cln-offers lightning-cli --network=regtest "$@"; }
node() { "${compose[@]}" exec -T cln lightning-cli --network=regtest "$@"; }
bitcoin() { "${compose[@]}" exec -T bitcoind bitcoin-cli -regtest -rpcuser=regtest -rpcpassword=regtest -rpcwallet=miner "$@"; }

"${compose[@]}" up -d --no-deps cln-offers
miner=$(bitcoin getnewaddress)
tip=$(bitcoin getblockcount)
deadline=$((SECONDS + 120))
until peer getinfo 2>/dev/null | python3 -c 'import json,sys; sys.exit(json.load(sys.stdin)["blockheight"] < int(sys.argv[1]))' "${tip}" 2>/dev/null; do
  if (( SECONDS >= deadline )); then echo "BOLT12 peer did not sync to bitcoind" >&2; exit 1; fi
  bitcoin generatetoaddress 1 "${miner}" >/dev/null
  sleep 2
done
id=$(node getinfo | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
peer connect "${id}@cln:9736" >/dev/null
if ! peer listpeerchannels "${id}" | python3 -c 'import json,sys; sys.exit(not any(c["state"] in ("CHANNELD_NORMAL","CHANNELD_AWAITING_LOCKIN","DUALOPEND_AWAITING_LOCKIN") for c in json.load(sys.stdin)["channels"]))'; then
  address=$(peer newaddr bech32 | python3 -c 'import json,sys; print(json.load(sys.stdin)["bech32"])')
  bitcoin sendtoaddress "${address}" 0.03 >/dev/null
  bitcoin generatetoaddress 6 "${miner}" >/dev/null
  deadline=$((SECONDS + 60))
  until peer listfunds | python3 -c 'import json,sys; sys.exit(not any(o["status"] == "confirmed" for o in json.load(sys.stdin)["outputs"]))'; do
    if (( SECONDS >= deadline )); then echo "BOLT12 peer funding was not confirmed" >&2; exit 1; fi
    sleep 1
  done
  peer -k fundchannel id="${id}" amount=2000000 push_msat=1000000000msat >/dev/null
fi
deadline=$((SECONDS + 120))
until peer listpeerchannels "${id}" | python3 -c 'import json,sys; sys.exit(not any(c["state"] == "CHANNELD_NORMAL" and c["peer_connected"] for c in json.load(sys.stdin)["channels"]))'; do
  if (( SECONDS >= deadline )); then echo "BOLT12 peer channel did not become usable" >&2; exit 1; fi
  bitcoin generatetoaddress 1 "${miner}" >/dev/null
  sleep 2
done
