<div align="center">
  <img src="https://numeraire.fra1.cdn.digitaloceanspaces.com/development/LOGO_FULL_PNG_color_dark_9d58514132.png" height="200" alt="Numeraire"/>
  <h1>SwissKnife</h1>
  <p>A Bitcoin wallet and transaction orchestrator with a Rust API and a Next.js dashboard.</p>
</div>

[![GitHub stars](https://img.shields.io/github/stars/bitcoin-numeraire/swissknife.svg?style=social&label=Star)](https://github.com/bitcoin-numeraire/swissknife/stargazers)

SwissKnife connects your LND or Core Lightning node to accounts, wallets, and
payment integrations. It supports on-chain Bitcoin, Lightning, and instant
internal transfers, with a shared account model for hosted and self-hosted use.

See the [changelog](CHANGELOG.md) for release notes and the
[upgrade guide](docs/UPGRADING.md) before upgrading from `0.2.0`.

## Features

- Accounts with authentication identities, preferences, and multiple wallets.
  Each wallet holds one asset on one settlement network.
- On-chain addresses, Lightning invoices, LNURL-pay, Lightning Addresses,
  Nostr NIP-05 verification, and Zaps.
- Internal transfers, balance reservations, and provider-derived payment fee quotes.
- Durable account settlement events over server-sent events, with reconnect replay
  and live dashboard updates.
- A dashboard for wallet activity, sending and receiving, contacts, identities,
  API keys, and account and wallet administration.
- OAuth2/OIDC with JWKS discovery and refresh, local JWT owner authentication,
  account-owned API keys, and permission-based administrative access.
- A documented REST API at `/docs`, with filtering and pagination.

The asset model distinguishes networks and can represent future asset protocols.
The current provider adapters settle native BTC; Taproot Assets, RGB, BOLT12,
webhooks, and desktop applications remain future work.

## Liquidity providers

SwissKnife supports LND and Core Lightning over gRPC or REST. You run the node
and manage its keys and liquidity; SwissKnife does not bundle a Lightning node.

SwissKnife wallet balances are independent of funds held by the provider.
Changing nodes does not rewrite those balances. Backing and liquidity are the
operator's responsibility, and an external payment can fail if the node lacks
liquidity even when its SwissKnife wallet has sufficient balance.

## Installation

The release workflow publishes three images for `linux/amd64` and `linux/arm64`:

| Image                                         | Contents                     | Container port |
| --------------------------------------------- | ---------------------------- | -------------- |
| `bitcoinnumeraire/swissknife:0.3.0`           | Backend and static dashboard | `3000`         |
| `bitcoinnumeraire/swissknife-server:0.3.0`    | Backend only                 | `3000`         |
| `bitcoinnumeraire/swissknife-dashboard:0.3.0` | Standalone Next.js dashboard | `8080`         |

Pin a release tag or digest for deployment. Configure the database, Lightning
provider, TLS credentials, and authentication before starting the backend.
Settings load from `config/default.toml`, an optional `config/<RUN_MODE>.toml`,
and `SWISSKNIFE_` environment variables. Nested settings use `__`, for example
`SWISSKNIFE_DATABASE__URL` and `SWISSKNIFE_LND_GRPC_CONFIG__ENDPOINT`.

The backend needs a persistent database. SQLite defaults to
`storage/swissknife.db`; mount `/storage` when using that default in a container.
For PostgreSQL, set `SWISSKNIFE_DATABASE__URL` to the database connection URL.
Mount provider certificates and credentials at their configured paths. For local
JWT authentication, set a strong `SWISSKNIFE_JWT__SECRET` before initial sign-up.

Dashboard `NEXT_PUBLIC_*` settings are compiled into its browser bundle. The
checked-in production profile targets Numeraire's hosted Auth0 deployment.
For your own deployment, edit `dashboard/.env.production` with your public API
and authentication settings before building the bundled or dashboard image;
changing container environment variables after the build does not change those
browser settings. See the [dashboard guide](dashboard/README.md).

From a configured source checkout, build the required image:

```bash
make build-docker            # Backend + static dashboard
make build-docker-server     # Backend only
make build-docker-dashboard  # Standalone dashboard
```

For Kubernetes, the [k8s-infra repository](https://github.com/bitcoin-numeraire/k8s-infra)
contains the backend and dashboard Helm charts. See the
[upgrade guide](docs/UPGRADING.md) for coordinated deployment and migration steps.

## Development

Use the Rust toolchain pinned in `rust-toolchain.toml`, Protocol Buffers compiler
(`protoc`), Docker Compose for integration tests, and Node 24 / Yarn 4 for the
dashboard.

```bash
make fmt
make lint
make test
make test-integration-fresh ITEST_DATABASE=postgres ITEST_PROVIDER=cln_rest
```

See [contributing](docs/CONTRIBUTING.md) for local setup and checks.

## Documentation

- [Public documentation](https://docs.numeraire.tech/swissknife)
- [Upgrading to 0.3.0](docs/UPGRADING.md)
- [Preparing and publishing a release](docs/RELEASING.md)
- [Dashboard development and configuration](dashboard/README.md)
- [Database migrations](crates/migration/README.md)
- [Developer guidelines](docs/DEVELOPER_GUIDELINES.md)
- [Integration tests](tests/itest/README.md)
- Architecture decisions:
  - [0001: AppStore, Unit of Work, and wallet reservations](docs/adr/0001-appstore-uow-wallet-reservations.md)
  - [0002: Identity, assets, and asset-scoped wallets](docs/adr/0002-identity-assets-wallet-model.md)
  - [0003: Provider fee quotes and fee limits](docs/adr/0003-provider-fee-quotes.md)
  - [0004: Durable client events and SSE](docs/adr/0004-client-event-outbox.md)
- [Code of conduct](docs/CODE_OF_CONDUCT.md)
