# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

SwissKnife is a self-custodial Bitcoin wallet and transaction orchestrator supporting Lightning Network, LNURL, Lightning Addresses, and Nostr protocols. It consists of a Rust backend API and a Next.js dashboard.

## Build & Development Commands

### Backend (Rust)
```bash
cargo build                    # Build
cargo run                      # Run server
make watch                     # Hot reload with cargo-watch
make lint                      # Run clippy linter
make fmt                       # Check formatting
make fmt-fix                   # Fix formatting
```

### Dashboard (Next.js)
```bash
cd dashboard
yarn install
yarn dev                       # Runs on port 8080
yarn build
yarn lint
yarn lint:fix
yarn fm:fix                    # Format with prettier
yarn openapi-ts                # Regenerate API client from OpenAPI spec
```

### Database Migrations
```bash
make new-migration name=migration-name   # Create new migration
make run-migrations                       # Run migrations
make generate-models                      # Regenerate Sea-ORM models from schema
```

You can generate new migrations using `make new-migration name=migration-name`.
See other migrations under the `./migration` folder.
Once a migration is added, you can work on it by modifying the file and then adding it in the `lib.rs` file.
The models are generated based on the schema and not the other way around.

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
- `dtos/` - Data transfer objects for API requests/responses
- `entities/` - App-wide entities (AppStore, AppServices, LnNodeClient)
- `errors/` - Error types (ApplicationError, AuthenticationError, etc.)
- `docs/` - OpenAPI documentation

**domains/** - Business logic organized by domain
- Each domain (invoice, payment, wallet, ln_address, lnurl, user, system, nostr) contains:
  - `*_handler.rs` - Axum route handlers
  - `*_service.rs` - Business logic implementation
  - `*_use_cases.rs` - Trait defining use cases
  - `*_repository.rs` - Data access trait
  - `entities/` - Domain entities

**infra/** - Infrastructure implementations
- `app/` - AppState initialization and Axum server setup
- `axum/` - Axum configuration and types
- `database/sea_orm/` - Sea-ORM client and models
- `lightning/` - Lightning node clients (Breez, CLN gRPC/REST, LND)
- `jwt/` - Authentication (local JWT, OAuth2)
- `logging/` - Tracing setup

### Configuration

Configuration files in `config/`:
- `default.toml` - Base configuration
- `development.toml` - Development overrides
- `production.toml` - Production overrides

Environment variable prefix: `SWISSKNIFE_` (e.g., `SWISSKNIFE_DATABASE__URL`)

Set `RUN_MODE=development|production` to select config profile.

### Lightning Providers

Configured via `ln_provider` in config:
- `breez` - Breez SDK
- `cln_grpc` - Core Lightning gRPC
- `cln_rest` - Core Lightning REST
- `lnd` - LND REST

### API Routes (server.rs)

- `/.well-known/lnurlp/:username` - LNURL-pay
- `/.well-known/nostr.json` - NIP-05
- `/v1/system` - Health and info
- `/v1/invoices` - Invoice management
- `/v1/payments` - Payment management
- `/v1/wallets` - Wallet management
- `/v1/me` - Current user wallet
- `/v1/auth` - Authentication
- `/v1/api-keys` - API key management
- `/v1/lightning-addresses` - Lightning address management
- `/lnurlp` - LNURL-pay callbacks
- `/docs` - Scalar API documentation

## Key Dependencies

- **Rust 1.87** (see rust-toolchain.toml)
- **Axum** - Web framework
- **Sea-ORM** - Database ORM (SQLite/PostgreSQL)
- **tokio** - Async runtime
- **tracing** - Logging
- **utoipa** - OpenAPI generation
- **Dashboard**: Next.js 14, MUI, TypeScript, Yarn 4, Node 20

## Code Review Guidelines

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

### Architecture
- Handlers should only handle HTTP concerns; business logic belongs in services
- Repository traits should be used for data access, not direct database calls in services
- DTOs should be used for API boundaries; domain entities for internal logic
- New routes must be documented with utoipa annotations for OpenAPI spec

### Security
- Authentication middleware must be applied to protected routes
- User input must be validated before processing
- Sensitive data (keys, tokens) must not be logged
- Check for proper authorization (user can only access their own resources)
