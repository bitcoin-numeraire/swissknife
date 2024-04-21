COMPOSE := docker compose -f docker-compose.yml
EXPOSED_PORTS := 50001 50002
BCLI := $(COMPOSE) exec -T -u blits bitcoind bitcoin-cli -regtest
DB_SERVICE := postgres
PGADMIN_SERVICE := pgadmin

.PHONY: up up-bitcoin up-electrs up-proxy up-postgres up-pgadmin shutdown down mine send create-wallet generate-certs

up:
	@$(MAKE) shutdown
	@$(MAKE) up-bitcoin
	@$(MAKE) create-wallet name=miner
	@$(MAKE) mine wallet=miner blocks=150
	@$(MAKE) up-electrs
	@$(MAKE) up-proxy
	@$(MAKE) up-postgres

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
	@$(COMPOSE) up -d electrs
	until $(COMPOSE) logs electrs | grep 'finished full compaction'; do sleep 1; done

up-proxy:
	@$(COMPOSE) up -d proxy
	until $(COMPOSE) logs proxy | grep 'App is running at http://localhost:3000'; do sleep 1; done

up-postgres:
	@$(COMPOSE) up -d $(DB_SERVICE)
	@until $(COMPOSE) logs $(DB_SERVICE) | grep 'database system is ready to accept connections'; do sleep 1; done
	@sea-orm-cli migrate up
	@sea-orm-cli generate entity -o src/application/models

up-pgadmin:
	@$(COMPOSE) up -d $(PGADMIN_SERVICE)
	@until $(COMPOSE) logs $(PGADMIN_SERVICE) | grep 'pgAdmin 4 - Application Initialisation'; do sleep 1; done

down:
	@$(COMPOSE) down

shutdown:
	@$(COMPOSE) down -v
	@rm -rf storage/rgblib/*

create-wallet:
	@$(BCLI) createwallet $(name)

mine:
	@$(BCLI) -rpcwallet=$(wallet) -generate $(blocks)

send:
	$(BCLI) -rpcwallet=miner sendtoaddress $(recipient) $(amount)
	@$(BCLI) -rpcwallet=$(wallet) -generate 4

install-tools:
	@cargo install sea-orm-cli

generate-certs:
	@mkdir -p certs
	@openssl genrsa -out certs/localhost_key.pem 2048
	@openssl req -new -x509 -key certs/localhost_key.pem -out certs/localhost_cert.pem -days 365 -subj /CN=localhost
