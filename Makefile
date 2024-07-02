COMPOSE := docker compose -f docker-compose.yml
DB_SERVICE := postgres
PGADMIN_SERVICE := pgadmin
LIGHTNINGD_SERVICE := lightningd
IMAGE_NAME := swissknife:latest

.PHONY: up up-lightningd up-postgres up-pgadmin shutdown down generate-certs build-docker run-docker

up:
	@$(MAKE) down
	@$(MAKE) up-postgres

up-lightningd:
	@$(COMPOSE) up -d $(LIGHTNINGD_SERVICE)
	@until $(COMPOSE) logs $(LIGHTNINGD_SERVICE) | grep 'lightningd: Server started'; do sleep 1; done

up-postgres:
	@$(COMPOSE) up -d $(DB_SERVICE)
	@until $(COMPOSE) logs $(DB_SERVICE) | grep 'database system is ready to accept connections'; do sleep 1; done

up-pgadmin:
	@$(COMPOSE) up -d $(PGADMIN_SERVICE)
	@until $(COMPOSE) logs $(PGADMIN_SERVICE) | grep 'pgAdmin 4 - Application Initialisation'; do sleep 1; done

down:
	@$(COMPOSE) down

shutdown:
	@$(COMPOSE) down -v
	@rm -rf storage/rgblib/*
	@rm -rf deps/lightningd/data/*

install-tools:
	@cargo install sea-orm-cli
	@cargo install cargo-edit
	@cargo install cargo-outdated

generate-models:
	@sea-orm-cli generate entity --output-dir models --verbose 

generate-certs:
	@mkdir -p certs
	@openssl genrsa -out certs/client_key.pem 2048
	@openssl req -new -x509 -key certs/client_key.pem -out certs/client_cert.pem -days 365 -subj /CN=localhost

build-docker:
	@docker build -t $(IMAGE_NAME) .

run-docker:
	@docker run -p 5000:5000 $(IMAGE_NAME)

deps-upgrade:
	@cargo upgrade

deps-outdated:
	@cargo outdated

lint:
	@cargo clippy

lint-fix:
	@cargo fix