# Swissknife Integration Tests

Integration tests that exercise SwissKnife through its **public HTTP API only**.
Each test runs against the real compiled binary, spawned by the harness against
a dockerized regtest stack (bitcoind + LND + CLN + Postgres). Tests never call
SwissKnife services or repositories directly.

## Layout

```text
tests/
  api.rs            entry point (gated behind the `itest` feature)
  common/           harness: process spawn, HTTP client, fixtures, assertions, db
  suites/           one module per API domain; tests grouped + named like the
                    unit tests (`mod <endpoint> { fn <case> }`)
  itest/
    config/         per-daemon config files (bitcoin/, lnd/, cln/), mounted read-only
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
make itest-shutdown                # stop stack + delete runtime/artifacts
```

`make test-integration` brings the stack up first, then runs
`cargo test --features itest --test api` for the selected cell. A plain
`cargo test` runs only unit tests (the suite is feature-gated).

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

## Isolation model

One shared SwissKnife instance per `(database, provider)` cell; tests isolate by
creating uniquely-named entities and asserting on presence rather than global
totals. The admin user is provisioned once during startup.

## Coverage

```bash
make coverage-lcov     # merged unit + integration coverage -> lcov.info
```

## Artifacts

SwissKnife stdout/stderr per cell is written under `target/itest/`. On CI
failure, dependency logs are collected via `make itest-logs`.

## Status / TODO

- Covered: system, auth (local JWT), wallet management, validation/auth/error paths.
- Pending: lightning invoice/pay/receive over a funded channel (needs an external
  counterparty node in the topology), the full provider matrix, and OAuth2.
- Found while writing these tests: bitcoin-numeraire/swissknife#254 (wallet
  auto-provisioning on first login races under concurrency).
