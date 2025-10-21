COMPOSE := docker compose -f docker-compose.yml
DB_SERVICE := postgres
PGADMIN_SERVICE := pgadmin
SWISSKNIFE_SERVICE := swissknife
IMAGE_NAME := swissknife:latest

.PHONY: watch up up-postgres up-pgadmin shutdown down generate-certs build-docker build-docker-server build-docker-dashboard run-docker lint fmt fmt-fix deps-upgrade deps-outdated install-tools generate-models new-migration

watch:
	@cargo watch -x run

up:
	@$(MAKE) down
	@$(MAKE) up-postgres
	@$(MAKE) up-swissknife

up-swissknife:
	@$(COMPOSE) up -d $(SWISSKNIFE_SERVICE)
	@until $(COMPOSE) logs $(SWISSKNIFE_SERVICE) | grep 'Listening on'; do sleep 1; done

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

install-tools:
	@cargo install cargo-watch
	@cargo install sea-orm-cli
	@cargo install cargo-edit
	@cargo install cargo-outdated

generate-models:
	@sea-orm-cli generate entity --output-dir src/infra/database/sea_orm/models

generate-certs:
	@mkdir -p certs
	@openssl genrsa -out certs/client_key.pem 2048
	@openssl req -new -x509 -key certs/client_key.pem -out certs/client_cert.pem -days 365 -subj /CN=localhost

build-docker:
	@docker build -t $(IMAGE_NAME) .

build-docker-server:
	@docker build -t swissknife-server:latest --target swissknife-server .

build-docker-dashboard:
	@docker build -t swissknife-dashboard:latest -f dashboard/Dockerfile ./dashboard

run-docker:
	@docker run -p 5000:5000 $(IMAGE_NAME)

deps-upgrade:
	@cargo upgrade --verbose

deps-outdated:
	@cargo outdated

lint:
	@cargo clippy --workspace --all-targets -- --no-deps

fmt:
	@cargo fmt --all -- --check

fmt-fix:
	@cargo fmt --all

new-migration:
	@sea-orm-cli migrate generate $(name)

run-migrations:
	@sea-orm-cli migrate up

fresh-migrations:
	@sea-orm-cli migrate fresh 