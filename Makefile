COMPOSE := docker compose -f docker-compose.yml
EXPOSED_PORTS := 50001 50002
DB_SERVICE := postgres
PGADMIN_SERVICE := pgadmin

.PHONY: up up-postgres up-pgadmin shutdown down generate-certs

up:
	@$(MAKE) down
	@$(MAKE) up-postgres

up-postgres:
	@$(COMPOSE) up -d $(DB_SERVICE)
	@until $(COMPOSE) logs $(DB_SERVICE) | grep 'database system is ready to accept connections'; do sleep 1; done
	@sea-orm-cli migrate up

up-pgadmin:
	@$(COMPOSE) up -d $(PGADMIN_SERVICE)
	@until $(COMPOSE) logs $(PGADMIN_SERVICE) | grep 'pgAdmin 4 - Application Initialisation'; do sleep 1; done

down:
	@$(COMPOSE) down

shutdown:
	@$(COMPOSE) down -v
	@rm -rf storage/rgblib/*

install-tools:
	@cargo install sea-orm-cli

generate-models:
	@sea-orm-cli generate entity --output-dir models --verbose 

generate-certs:
	@mkdir -p certs
	@openssl genrsa -out certs/client_key.pem 2048
	@openssl req -new -x509 -key certs/client_key.pem -out certs/client_cert.pem -days 365 -subj /CN=localhost
