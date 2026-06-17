COMPOSE := docker compose -f docker-compose.yml
DB_SERVICE := postgres
PGADMIN_SERVICE := pgadmin
SWISSKNIFE_SERVICE := swissknife
SWISSKNIFE_SERVER_SERVICE := swissknife-server
IMAGE_NAME := swissknife:latest
ITEST_PROJECT ?= swissknife-itest
ITEST_COMPOSE = docker compose -p $(ITEST_PROJECT) -f tests/itest/docker-compose.yml
ITEST_DATABASE ?= sqlite
ITEST_PROVIDER ?= lnd_grpc
# Optional test-name filter (TEST) and post-`--` args (TESTARGS) for the test
# targets, so a single suite or test can be run locally, e.g.:
#   make test-integration TEST=suites::oauth2
#   make test-integration TEST=suites::lnurl_send::pay TESTARGS="--nocapture"
#   make test-unit TEST=payment_service
TEST ?=
TESTARGS ?=
# Per-instance dynamics read by the harness; everything static lives in config/itest.toml.
ITEST_ENV = SWISSKNIFE_ITEST_COMPOSE_PROJECT=$(ITEST_PROJECT) \
	SWISSKNIFE_ITEST_DATABASE=$(ITEST_DATABASE) \
	SWISSKNIFE_ITEST_PROVIDER=$(ITEST_PROVIDER)

.PHONY: watch up up-swissknife up-server up-postgres up-pgadmin shutdown down generate-certs build build-docker build-docker-server build-docker-dashboard run-docker lint fmt fmt-fix test test-unit test-integration test-persistence itest-up itest-down itest-shutdown itest-logs coverage coverage-html coverage-lcov coverage-matrix clean check deps-upgrade deps-outdated install-tools generate-models new-migration run-migrations fresh-migrations

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
	@rm -rf storage/regtest

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
	@cargo clippy --workspace --all-targets --features itest --no-deps -- -D warnings

fmt:
	@cargo fmt --all -- --check

fmt-fix:
	@cargo fmt --all

build:
	@cargo build --workspace --all-targets --features itest

test:
	@cargo test --workspace --all-targets

test-unit:
	@cargo test --workspace --bins $(TEST) -- $(TESTARGS)

# Run the integration suite for one (database, provider) cell. Brings
# the regtest stack up first (idempotent); override the cell via
# ITEST_DATABASE / ITEST_PROVIDER (e.g. `make test-integration ITEST_PROVIDER=cln_grpc`).
# Narrow to a suite/test with TEST=... and pass runner flags with TESTARGS=...
test-integration: itest-up
	@$(ITEST_ENV) cargo test --features itest --test api $(TEST) -- $(TESTARGS)

# Run the persistence / Unit-of-Work tests for one database cell: real-DB
# coverage of the reservation/settlement balance invariants and concurrency.
# SQLite is self-contained; postgres uses the dockerized PG. Override the cell
# via ITEST_DATABASE (sqlite|postgres); TEST defaults to the uow_tests module.
test-persistence:
	@if [ "$(ITEST_DATABASE)" = "postgres" ]; then $(ITEST_COMPOSE) up -d --wait postgres; fi
	@$(ITEST_ENV) cargo test --features itest --bins $(if $(TEST),$(TEST),uow_tests) -- $(TESTARGS)

# Bring up the regtest dependency stack (bitcoind + LND + CLN + Postgres).
itest-up:
	@SWISSKNIFE_ITEST_COMPOSE_PROJECT=$(ITEST_PROJECT) tests/itest/scripts/bootstrap.sh

# Stop the dependency stack, keeping volumes.
itest-down:
	@$(ITEST_COMPOSE) down

# Stop the dependency stack and delete all integration runtime/artifacts.
itest-shutdown:
	@$(ITEST_COMPOSE) down -v
	@rm -rf tests/itest/runtime target/itest lcov.info

# Collect dependency logs into target/itest/dependency-logs.
itest-logs:
	@SWISSKNIFE_ITEST_COMPOSE_PROJECT=$(ITEST_PROJECT) tests/itest/scripts/collect-logs.sh target/itest/dependency-logs

# Unit-test coverage via cargo-llvm-cov (install with `make install-tools`).
coverage:
	@cargo llvm-cov --workspace --bins

coverage-html:
	@cargo llvm-cov --workspace --bins --html
	@echo "HTML coverage report generated at target/llvm-cov/html/index.html"

# Combined unit + integration coverage for ONE cell -> lcov.info. Requires the
# regtest stack (brought up here); merges unit + UoW (`--features itest` so the
# itest-gated uow_tests are compiled) with the integration suite for the selected
# (ITEST_DATABASE, ITEST_PROVIDER) cell, including the spawned binary's coverage.
coverage-lcov: itest-up
	@cargo llvm-cov clean --workspace
	@cargo llvm-cov --no-report --workspace --features itest --bins
	@$(ITEST_ENV) cargo llvm-cov --no-report --features itest --test api
	@cargo llvm-cov report --lcov --output-path lcov.info

# Full-matrix merged coverage: unit + UoW (both DBs) + the integration API suite
# across every cell in COVERAGE_CELLS, merged into one report. This is the true
# total — run it on Linux/CI (lnd_rest does not start under macOS native-tls).
# Integration cells run serially (--test-threads=1) to avoid the SQLite write
# contention in #267. Override COVERAGE_CELLS to add/drop cells.
COVERAGE_CELLS ?= sqlite:lnd_grpc sqlite:lnd_rest sqlite:cln_grpc sqlite:cln_rest postgres:lnd_grpc
coverage-matrix: itest-up
	@cargo llvm-cov clean --workspace
	@cargo llvm-cov --no-report --workspace --features itest --bins
	@SWISSKNIFE_ITEST_DATABASE=postgres cargo llvm-cov --no-report --features itest --bins -- uow_tests
	@for cell in $(COVERAGE_CELLS); do \
		db=$${cell%%:*}; provider=$${cell##*:}; \
		echo ">>> integration $$db/$$provider"; \
		SWISSKNIFE_ITEST_COMPOSE_PROJECT=$(ITEST_PROJECT) SWISSKNIFE_ITEST_DATABASE=$$db SWISSKNIFE_ITEST_PROVIDER=$$provider \
			cargo llvm-cov --no-report --features itest --test api -- --test-threads=1 || exit 1; \
	done
	@cargo llvm-cov report --lcov --output-path lcov.info
	@cargo llvm-cov report

# Remove build artifacts, coverage output, and integration runtime/artifacts.
clean:
	@cargo clean
	@rm -f lcov.info
	@find . -name '*.profraw' -delete

check: fmt lint build test

new-migration:
	@sea-orm-cli migrate -d crates/migration generate $(name)

run-migrations:
	@sea-orm-cli migrate -d crates/migration up

fresh-migrations:
	@sea-orm-cli migrate -d crates/migration fresh

# Regenerate the dashboard's checked-in OpenAPI spec from the utoipa annotations,
# then regenerate the typed API client. Run after backend API changes.
openapi:
	@cargo test --quiet dump_openapi_spec -- --ignored
	@cd dashboard && yarn openapi-ts
