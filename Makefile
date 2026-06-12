COMPOSE := docker compose -f docker-compose.yml
DB_SERVICE := postgres
PGADMIN_SERVICE := pgadmin
SWISSKNIFE_SERVICE := swissknife
SWISSKNIFE_SERVER_SERVICE := swissknife-server
IMAGE_NAME := swissknife:latest

.PHONY: watch up up-swissknife up-server up-postgres up-pgadmin shutdown down generate-certs build build-docker build-docker-server build-docker-dashboard run-docker lint fmt fmt-fix test test-unit test-integration coverage coverage-html coverage-lcov check deps-upgrade deps-outdated install-tools generate-models new-migration run-migrations fresh-migrations

watch:
	@cargo watch -x run

up:
	@$(MAKE) down
	@$(MAKE) up-postgres
	@$(MAKE) up-swissknife

up-swissknife:
	@$(COMPOSE) up -d $(SWISSKNIFE_SERVICE)
	@until $(COMPOSE) logs $(SWISSKNIFE_SERVICE) | grep 'Listening on'; do sleep 1; done

up-server:
	@$(COMPOSE) up -d $(SWISSKNIFE_SERVER_SERVICE)
	@until $(COMPOSE) logs $(SWISSKNIFE_SERVER_SERVICE) | grep 'Listening on'; do sleep 1; done

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
	@rm -f storage/swissknife.db
	@rm -f storage/regtest

install-tools:
	@cargo install cargo-watch
	@cargo install sea-orm-cli
	@cargo install cargo-edit
	@cargo install cargo-outdated
	@cargo install cargo-llvm-cov

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
	@cargo clippy --workspace --all-targets --no-deps -- -D warnings

fmt:
	@cargo fmt --all -- --check

fmt-fix:
	@cargo fmt --all

build:
	@cargo build --workspace --all-targets

test:
	@cargo test --workspace --all-targets

test-unit:
	@cargo test --workspace --bins

test-integration:
	@if find tests -mindepth 1 -name '*.rs' 2>/dev/null | grep -q .; then \
		cargo test --workspace --tests; \
	else \
		echo "No integration tests found under tests/"; \
	fi

# Code coverage via cargo-llvm-cov (install with `make install-tools`).
# Coverage runs the unit-test suite (`--bins`) and reports line/region/function
# coverage. Use `coverage-html` for a browsable report and `coverage-lcov` to
# emit an lcov.info file for CI or editor integrations.
coverage:
	@cargo llvm-cov --workspace --bins

coverage-html:
	@cargo llvm-cov --workspace --bins --html
	@echo "HTML coverage report generated at target/llvm-cov/html/index.html"

coverage-lcov:
	@cargo llvm-cov --workspace --bins --lcov --output-path lcov.info

check: fmt lint build test

new-migration:
	@sea-orm-cli migrate -d crates/migration generate $(name)

run-migrations:
	@sea-orm-cli migrate -d crates/migration up

fresh-migrations:
	@sea-orm-cli migrate -d crates/migration fresh
