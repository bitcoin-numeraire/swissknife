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
