COMPOSE := docker compose -f docker-compose.yml
EXPOSED_PORTS := 50001 50002
BCLI := $(COMPOSE) exec -T -u blits bitcoind bitcoin-cli -regtest
DB_SERVICE := postgres
PGADMIN_SERVICE := pgadmin

.PHONY: up-bitcoin up-electrs up-postgres up-pgadmin down mine send create-wallet generate-certs

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

up-postgres:
	@$(COMPOSE) up -d $(DB_SERVICE)
	@until $(COMPOSE) logs $(DB_SERVICE) | grep 'database system is ready to accept connections'; do sleep 1; done
	@sqlx migrate run

up-pgadmin:
	@$(COMPOSE) up -d $(PGADMIN_SERVICE)
	@until $(COMPOSE) logs $(PGADMIN_SERVICE) | grep 'pgAdmin 4 - Application Initialisation'; do sleep 1; done

down:
	@$(COMPOSE) down -v

create-wallet:
	@$(BCLI) createwallet $(name)

mine:
	@$(BCLI) -rpcwallet=$(wallet) -generate $(blocks)

send:
	$(BCLI) -rpcwallet=miner sendtoaddress $(recipient) $(amount)

install-tools:
	@cargo install sqlx-cli --no-default-features --features native-tls,postgres

generate-certs:
	@mkdir -p certs
	@openssl genrsa -out certs/localhost_key.pem 2048
	@openssl req -new -x509 -key certs/localhost_key.pem -out certs/localhost_cert.pem -days 365 -subj /CN=localhost
