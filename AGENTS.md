# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

SwissKnife is a self-custodial Bitcoin wallet and transaction orchestrator supporting Lightning Network, LNURL, Lightning Addresses, and Nostr protocols. It consists of a Rust backend API and a Next.js dashboard.

### Liquidity Providers and Wallet Balances

LND, CLN, and other nodes are liquidity providers. SwissKnife wallet balances are independent of the actual funds on those providers. Backing, solvency, and operational liquidity are the operator's responsibility; SwissKnife does not verify or enforce provider backing for its balances.

- Switching providers must not rewrite or cap SwissKnife balances. Moving from an LND node with 1,000,000 sats to a CLN node with 50,000 sats does not change the wallet balances.
- The product model permits authorized administrative credits above the provider's actual funds. Do not impose a node-balance cap or require an unallocated-reserves calculation for an administrative credit.
- A real node-funded on-chain transaction paying a registered SwissKnife deposit address follows the normal deposit lifecycle, including a node self-transaction. Credit the receiving wallet once on confirmation even though the provider gains no new backing. Ordinary node change addresses remain excluded.
- An out-of-band node transaction does not imply a debit from a SwissKnife wallet. Only settle an outgoing SwissKnife payment when an existing payment matches the provider transaction; do not fabricate a counterpart payment.
- Keep authorization, exact amounts, atomic balance/history/event writes, and replay protection correct within SwissKnife. Wallet spend checks still use SwissKnife balances, and an external payment may fail when provider liquidity is insufficient.
- A valid administrative credit or node self-transaction is not a backing warning. Use the normal audit/deposit logging for the operation.

## Build & Development Commands

### Backend (Rust)
```bash
cargo build                    # Build
cargo run                      # Run server
make watch                     # Hot reload with cargo-watch
make lint                      # Run clippy linter
make fmt                       # Check formatting
make fmt-fix                   # Fix formatting
make test                      # Run unit tests
make test-integration          # Run one black-box integration-test cell
make test-integration-fresh    # Recreate volumes before a liquidity-sensitive cell
make check                     # Format, lint, build, and unit tests
```

Before committing, ensure the linting and formatting pass successfully using `make lint` and `make fmt-fix`.

### Test Placement

- Cover new public endpoints and user-visible behavior with black-box integration tests under `tests/`. Exercise the compiled SwissKnife binary through its public API and include authentication, authorization, validation, persistence, and relevant real-provider behavior.
- Keep `src/infra/database/sea_orm/uow_tests.rs` for database transaction and concurrency guarantees, or behavior that cannot be exercised through the public API. Do not use it for ordinary repository CRUD or serialization round trips that an integration test can cover.

### Dashboard (Next.js)
```bash
cd dashboard
yarn install
yarn dev                       # Runs on port 8080
yarn build
yarn test
yarn lint
yarn lint:fix
yarn typecheck
yarn fm:fix                    # Format with prettier
yarn openapi-ts                # Regenerate API client from OpenAPI spec
```

Before committing, ensure the linting and formatting pass successfully using `yarn lint:fix` and `yarn fm:fix`.

### Database Migrations
```bash
make new-migration name=migration-name   # Create new migration
make run-migrations                       # Run migrations
make generate-models                      # Regenerate Sea-ORM models from schema
```

Generate migrations using `make new-migration name=migration-name`; do not hand-name migration files.
See other migrations under `crates/migration/src/`. Once a migration is added, implement it and register it in `lib.rs`.
The models are generated based on the schema and not the other way around.

To generate the models, the process is to start a postgres database with `make up-postgres`
and execute the migration against it with `make run-migrations`, then generate the models against the schema
with `make generate-models` with the ENV var `DATABASE_URL` as:
`postgres://postgres:postgres@localhost:5432/numeraire`. This requires Docker to work on the environment and
the postgres image needs to be downloaded, meaning that this is to be avoided as much as possible unless changes
in the models are required.

### Docker
```bash
make up-postgres               # Start postgres
make up                        # Start full stack
make down                      # Stop containers
make shutdown                  # Stop and remove volumes
```

## Architecture

### Three-Layer Structure (`src/`)

**application/** - Shared application concerns
- `composition/` - Configuration, adapter/service wiring, and shared application ports
- `errors/` - Error types (ApplicationError, AuthenticationError, etc.)
- `docs/` - OpenAPI documentation

**domains/** - Business logic organized by domain
- Each domain (account/auth, asset, invoice, payment, wallet, ln_address, bitcoin, lnurl, system, nostr) contains the applicable:
  - `*_handler.rs` - Axum route handlers
  - `*_service.rs` - Business logic implementation
  - `*_use_cases.rs` - Trait defining use cases
  - `*_repository.rs` - Data access trait
  - `entities/` - Domain entities

**infra/** - Infrastructure implementations
- `app/` - AppState initialization and Axum server setup
- `axum/` - Axum configuration and types
- `database/sea_orm/` - Sea-ORM client and models
- `lightning/` - Lightning node clients (CLN gRPC/REST, LND gRPC/REST)
- `jwt/` - Authentication (local JWT, OAuth2)
- `logging/` - Tracing setup

### Configuration

Configuration files in `config/`:
- `default.toml` - Required base configuration
- `<RUN_MODE>.toml` - Optional deployment/development overrides
- `itest.toml` - Integration-test overrides

Environment variable prefix: `SWISSKNIFE_` (e.g., `SWISSKNIFE_DATABASE__URL`)

Set `RUN_MODE=development|production` to select config profile.

### Lightning Providers

Configured via `ln_provider` in config:
- `cln_grpc` - Core Lightning gRPC
- `cln_rest` - Core Lightning REST
- `lnd_grpc` - LND gRPC
- `lnd_rest` - LND REST

### API Routes (server.rs)

- `/.well-known/lnurlp/:username` - LNURL-pay
- `/.well-known/nostr.json` - NIP-05
- `/v1/system` - Health and info
- `/v1/me` - Authenticated account profile, preferences, API keys, and explicitly selected account-wallet resources
- `/v1/invoices` - Administrative invoice management
- `/v1/payments` - Administrative payment management
- `/v1/wallets` - Administrative wallet management
- `/v1/auth` - Authentication
- `/v1/api-keys` - Administrative API key management
- `/v1/lightning-addresses` - Administrative Lightning Address management
- `/v1/bitcoin/addresses` - Administrative Bitcoin address management
- `/lnurlp` - LNURL-pay callbacks
- `/docs` - Scalar API documentation

## Key Dependencies

- **Rust 1.99.1** (see rust-toolchain.toml)
- **Axum** - Web framework
- **Sea-ORM** - Database ORM (SQLite/PostgreSQL)
- **tokio** - Async runtime
- **tracing** - Logging
- **utoipa** - OpenAPI generation
- **Dashboard**: Next.js 16, React 19, MUI 9, TypeScript, Yarn 4, Node 24

## Code Review Guidelines

### Changelog

- Update `CHANGELOG.md` for user-facing behavior changes. Documentation, agent guidance, tests, and internal refactors without user-facing changes do not require a changelog entry.
- Changelog CI validates the changelog's format; it does not require every PR to add an entry. Do not add an entry solely to satisfy that check.

### General
- Verify all tests pass (`cargo test`)
- Run `make lint` and `make fmt` before committing
- Check for security vulnerabilities (SQL injection, improper auth checks)
- Ensure error handling is consistent using `ApplicationError` types

### Rust-Specific
- Avoid `unwrap()` and `expect()` in production code paths; use proper error propagation with `?`
- Prefer `&str` over `String` for function parameters when ownership isn't needed
- Check for proper use of `async`/`await` and avoid blocking operations in async contexts
- Ensure `Clone` and `Copy` are only derived when necessary
- Validate that database transactions are properly scoped
- Follow `docs/DEVELOPER_GUIDELINES.md` for architecture, unit-test structure, mock usage, and behavior expectations

### Architecture
- Handlers should only handle HTTP concerns; business logic belongs in services
- Repository traits should be used for data access, not direct database calls in services
- Shared API entities and request/response types live in `crates/swissknife-types`
- New routes must be documented with utoipa annotations for OpenAPI spec
- Logging happens in use cases (services) and nowhere else in general except in  
  `infra` for listeners when they do not return errors and at the edges like `axum_response`.

### Security
- Authentication middleware must be applied to protected routes
- User input must be validated before processing
- Sensitive data (keys, tokens) must not be logged
- Check for proper authorization (user can only access their own resources)
