COMPOSE := docker compose -f docker-compose.yml
EXPOSED_PORTS := 50001 50002
BCLI := $(COMPOSE) exec -T -u blits bitcoind bitcoin-cli -regtest

.PHONY: up-bitcoin up-electrs down mine send create-wallet generate-certs

up-bitcoin:
	@make down
	@for port in $(EXPOSED_PORTS); do \
		if lsof -Pi :$$port -sTCP:LISTEN -t >/dev/null ; then \
			echo "port $$port is already bound, services can't be started"; \
 			exit 1; \
		fi \
	done
	@$(COMPOSE) up -d bitcoind
	until $(COMPOSE) logs bitcoind | grep 'Bound to'; do sleep 1; done

up-electrs:
	@$(COMPOSE) up -d electrs electrs-2
	until $(COMPOSE) logs electrs | grep 'finished full compaction'; do sleep 1; done
	until $(COMPOSE) logs electrs-2 | grep 'finished full compaction'; do sleep 1; done

down:
	@$(COMPOSE) down -v

create-wallet:
	@$(BCLI) createwallet $(name)

mine:
	@$(BCLI) -rpcwallet=$(wallet) -generate $(blocks)

send:
	$(BCLI) -rpcwallet=miner sendtoaddress $(recipient) $(amount)

generate-certs:
	@mkdir -p certs
	@openssl genrsa -out certs/localhost_key.pem 2048
	@openssl req -new -x509 -key certs/localhost_key.pem -out certs/localhost_cert.pem -days 365 -subj /CN=localhost
