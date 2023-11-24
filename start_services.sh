#!/bin/bash
set -eu

_die () {
    echo "ERR: $*"
    exit 1
}

COMPOSE="docker compose"
if ! $COMPOSE >/dev/null; then
    echo "could not call docker compose (hint: install docker compose plugin)"
    exit 1
fi
COMPOSE="$COMPOSE -f docker-compose.yml"

$COMPOSE down -v

# see docker-compose.yml for the exposed ports
EXPOSED_PORTS=(50001 50002)
for port in "${EXPOSED_PORTS[@]}"; do
    if [ -n "$(ss -HOlnt "sport = :$port")" ];then
        _die "port $port is already bound, services can't be started"
    fi
done
$COMPOSE up -d

# wait for bitcoind to be up
until $COMPOSE logs bitcoind |grep 'Bound to'; do
    sleep 1
done

# prepare bitcoin funds
BCLI="$COMPOSE exec -T -u blits bitcoind bitcoin-cli -regtest"
$BCLI createwallet miner
$BCLI -rpcwallet=miner -generate 111

# wait for electrs to have completed startup
until $COMPOSE logs electrs |grep 'finished full compaction'; do
    sleep 1
done
until $COMPOSE logs electrs-2 |grep 'finished full compaction'; do
    sleep 1
done
