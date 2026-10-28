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

Every compose host port is overridable with a `SWISSKNIFE_ITEST_*_PORT`
variable. When another regtest environment is already using the defaults, also
override the corresponding SwissKnife endpoint (for example
`SWISSKNIFE_LND_GRPC_CONFIG__ENDPOINT`) and the test helper endpoint
(`SWISSKNIFE_ITEST_BITCOIN_RPC_URL` or the Postgres URLs).

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
## LDK Server and Wavelength

`make test-integration ITEST_PROVIDER=ldk_server` builds the pinned LDK Server
Docker image, provisions a real LND/LDK channel and on-chain anchor reserve, and
runs `tests/providers.rs`. This Lightning-only contract shares the account,
auth, permissions, invoice and LNURL suites, tests sends and restart replay,
and explicitly asserts that unsupported on-chain operations fail. The existing
LND/CLN cells continue to run the full `tests/api.rs` suite.
LDK also peers with CLN for gossip and waits for both directional LND/CLN
routing policies. It has no direct CLN channel, so the fee test still routes
through LND and verifies a real forwarding fee.

`tests/itest/scripts/test-wavelength-client.sh` builds the pinned wallet-enabled
`waved` image and runs the independent crate's authenticated RPC tests. This is
a fresh, uninitialized-daemon transport test, **not** a funded Ark integration
test. Public signet/testnet interoperability and funded wallet coverage remain
#389. Numeraire tests must use the public daemon and documented public services;
internal operator source, images or employee credentials are outside scope. Both image
builds pin source commits in `build-provider-image.sh`; state and source caches
live under the ignored `runtime/` directory.

The test-only certificate/key under `crates/wavelength-client/tests/fixtures`
is public fixture data. It must never be used for an installation. An explicit
server certificate avoids the pinned daemon's `CA:TRUE` certificate generation
interoperability problem without disabling TLS verification.

`tests/itest/scripts/test-wavelength-public.sh` runs an **unfunded**, ephemeral
wallet against waved's documented public signet defaults. Set
`WAVELENGTH_PUBLIC_NETWORK=testnet` for testnet3. It checks wallet creation,
readiness, zero spendable balances, boarding-address creation, unpaid invoice
creation, and durable receive activity and identity after restarting and
unlocking the same daemon. Only `waved` runs
locally; its outbound TLS connections use public defaults and system trust.
No internal operator source, image, configuration or employee credential is
needed. The generated mnemonic is never printed or persisted by the test;
the container and temporary RPC credentials are removed on exit. Never import
a funded seed into this disposable suite.

The integration workflow's manual `wavelength_public_network` input selects
this live smoke test. It is separate from required PR checks because it depends
on shared public service availability. **It does not test funded payments.**
Funded Lightning send/receive, exact fees, confirmed boarding, restart settlement
and recovery still require test coins and a public payment counterparty under
#389 before Wavelength can become the default (#390). Public-network tests
cannot provide local block timing or deterministic fault injection; a public
black-box development sandbox would improve that coverage without exposing
operator internals.

`cargo test --features itest --test wavelength` tests the compiled SwissKnife
HTTP API, real persistence and independently implemented public wallet RPCs
over verified TLS. `SWISSKNIFE_ITEST_DATABASE=postgres` selects the second
accounting matrix cell (start the test Postgres first). It covers receive IDs
that differ from hashes, replay/duplicate protection, gross-outflow fee mapping,
incomplete/invalid quotes, credit top-up rejection, lost replies, wallet-local
timeouts, invalid settlement proofs, fee overruns and completed refunds.
These are deterministic contract tests; they are not funded Ark tests.

The public-network script also runs `tests/wavelength_public.rs` through the
compiled SwissKnife binary. It creates a real unpaid invoice with the public
services, validates the actual provider expiry/network, checks unsupported
custom expiry and verifies persistence after restarting SwissKnife. It moves
no funds and does not satisfy the funded default-cutover gate.
## Funded public Wavelength tests

`tests/itest/scripts/test-wavelength-funded.sh` starts two fresh public `waved`
wallets and prints a signet boarding address. Fund it with at least 100,000 test
sats using a public faucet or another test wallet. The runner waits up to twenty
minutes for spendable VTXOs, then receives into SwissKnife while it is offline,
sends back to the public counterparty with a hard fee limit, and restarts both
daemons and SwissKnife to verify exact fees, durable proofs and single ledger
credit/debit. It never imports or prints a seed, and rejects mainnet.

The wallets use the documented public network presets; no operator source,
private images or employee credentials are used. `WAVELENGTH_PUBLIC_NETWORK=testnet`
selects testnet3. `SWISSKNIFE_ITEST_DATABASE=postgres` runs the same API lifecycle
against PostgreSQL; SQLite is the default. Wallet-to-wallet payments can settle
on the same Ark operator, so this is not proof of external Lightning routing.
The observed settlement rail and fee are printed rather than inferred.

Wallet volumes and the private test state directory survive success, failure and
interruption. The exit message prints the command to resume the same wallets.
Resuming unlocks once, preserves identity and creates new invoices; it never
resubmits an earlier payment. After collecting recovery evidence, remove this
specific test project with Docker Compose `down -v` and delete its printed state
directory. These wallets use a fixed test-only password and must never hold real
funds. The **Integration Tests** workflow provides a `wavelength_funded` manual input
with SQLite/PostgreSQL cells (also select a public network); fund each cell's printed address during its
funding wait. This is a manual live-network gate, separate from deterministic PR
checks, and is only successful after the entire payment/restart scenario passes.
