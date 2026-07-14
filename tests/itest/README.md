# Swissknife Integration Tests

Integration tests that exercise SwissKnife through its **public HTTP API only**.
Each test runs against the real compiled binary, spawned by the harness against
a dockerized regtest stack (bitcoind + LND + CLN + Postgres + a mock OpenID
provider). Tests never call SwissKnife services or repositories directly.

External dependencies that are not the point of a test — or where a test asserts
the *outbound* request SwissKnife makes — are mocked: an external LNURL-pay
service (`wiremock`, in-process per test) and an OpenID provider
([`navikt/mock-oauth2-server`](https://github.com/navikt/mock-oauth2-server), a
compose service). Everything else is a real backend in regtest.

## Layout

```text
tests/
  api.rs            entry point (gated behind the `itest` feature)
  common/           harness: process spawn, HTTP client, fixtures, assertions, db
  suites/           one module per API domain; tests grouped + named like the
                    unit tests (`mod <endpoint> { fn <case> }`)
  itest/
    config/         per-daemon config files (bitcoin/, lnd/, cln/, mock-oauth2/), mounted read-only
    docker-compose.yml
    scripts/        bootstrap.sh (bring up + init), collect-logs.sh
    runtime/        generated state — chain data, certs, macaroons, rune (gitignored)
config/itest.toml   all static SwissKnife test config (RUN_MODE=itest)
```

## Running

```bash
make itest-up                      # bring up + initialize the regtest stack (idempotent)
make test-integration              # default cell: sqlite + lnd_grpc
make test-integration ITEST_DATABASE=postgres ITEST_PROVIDER=cln_grpc
make test-integration-fresh        # recreate volumes, then run sqlite + lnd_grpc
make itest-shutdown                # stop stack + delete runtime/artifacts
```

`make test-integration` brings the stack up first, then runs
`cargo test --features itest --test api` for the selected cell. A plain
`cargo test` runs only unit tests (the suite is feature-gated).

Use `make test-integration-fresh` before a local liquidity-sensitive run when
the previous channel balances or chain state are unknown. The fresh target runs
test cases serially to avoid unrelated SQLite writers contending for the same
database; concurrency scenarios still issue simultaneous requests inside their
test case. CI jobs start with a new runner and therefore already get fresh
dependency volumes.

## Persistence (Unit-of-Work) tests

The reservation/settlement balance invariants and their concurrency guarantees
(bitcoin-numeraire/swissknife#240) are covered by in-crate tests that drive the
Unit-of-Work types and their repositories against a **real** database — not the
mock — so the conditional `UPDATE`s that serialize concurrent settlements are
actually exercised. They live in
[`src/infra/database/sea_orm/uow_tests.rs`](../../src/infra/database/sea_orm/uow_tests.rs),
gated behind the same `itest` feature, and run on both backends:

```bash
make test-persistence ITEST_DATABASE=sqlite     # self-contained (temp sqlite file)
make test-persistence ITEST_DATABASE=postgres   # brings up + waits on the compose Postgres
```

SQLite needs no services; the Postgres cell brings up (and health-waits on) the
compose Postgres. Both run in CI. Each test provisions its own fresh database and
applies the full migration set, so they are independent and parallel-safe.

## Configuration

Everything static is in [`config/itest.toml`](../../config/itest.toml). The
harness injects only the per-instance dynamics as env vars:

| Variable | Purpose |
| --- | --- |
| `SWISSKNIFE_WEB__ADDR` | ephemeral `127.0.0.1:<port>` |
| `SWISSKNIFE_DATABASE__URL` | per-instance sqlite file / postgres db |
| `SWISSKNIFE_LN_PROVIDER` | backend under test |
| `SWISSKNIFE_CLN_REST_CONFIG__RUNE` | generated rune (cln_rest only) |

The matrix dimensions are selected via `SWISSKNIFE_ITEST_DATABASE`
(`sqlite` \| `postgres`) and `SWISSKNIFE_ITEST_PROVIDER`
(`lnd_grpc` \| `lnd_rest` \| `cln_grpc` \| `cln_rest`).

### OAuth2 / OIDC

The `oauth2` suite spins up a *second* instance configured with
`auth_provider = oauth2`, pointed at the dockerized mock OpenID provider. The
binary and the tests reach the IdP at the same `127.0.0.1:<port>` (default
`8090`, override with `SWISSKNIFE_ITEST_OAUTH2_PORT`), so the issuer SwissKnife
discovers matches the `iss` the IdP stamps into tokens. Token claim sets are
shaped per request `client_id` by `config/mock-oauth2/config.json`, whose
audience must match the harness-set `SWISSKNIFE_OAUTH2__AUDIENCE`.

## Isolation model

One shared SwissKnife instance per `(database, provider)` cell (plus the shared
OAuth2 instance once the oauth2 suite runs); each gets its own database. Tests
isolate by creating uniquely-named entities and asserting on presence rather
than global totals.

Fixtures are explicit and use the same public API as clients: the harness signs
up the local admin, while behavioral tests create their account with
`POST /v1/accounts`, mint an account API key, and create the required asset
wallets before exercising `/v1/me`. OAuth2 tests deliberately provision their
subjects on the request under test, including simultaneous first requests. No
account, identity, or wallet rows are inserted directly by the test harness.

The `asset_scoping` suite creates mainnet, regtest, and signet BTC wallets for
one account. It verifies that balances, resources, reservations, and overview
aggregates stay attached to the concrete wallet and that the regtest runtime
rejects invoice/payment operations through incompatible network wallets.

## Coverage

```bash
make coverage-lcov     # merged unit + integration coverage -> lcov.info
```

## Artifacts

SwissKnife stdout/stderr per cell is written under `target/itest/`. On CI
failure, dependency logs are collected via `make itest-logs`.
