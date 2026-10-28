# Contributing

SwissKnife is implemented in Rust with the toolchain pinned in
`rust-toolchain.toml`. Install `protoc` for the generated Lightning clients.
Docker Compose supplies local PostgreSQL and integration-test dependencies.
To run the backend locally against PostgreSQL:

```bash
make up-postgres
SWISSKNIFE_DATABASE__URL=postgres://postgres:postgres@localhost:5432/numeraire cargo run
```

The backend applies migrations at startup. Configure the selected Lightning
provider, its credentials, and authentication in `config/development.toml` or
`SWISSKNIFE_` environment variables first. `config/default.toml` lists the
available settings; nested environment keys use `__`.

For a fresh local JWT installation, choose a strong `SWISSKNIFE_JWT__SECRET`.
The initial `/v1/auth/sign-up` creates the owner login. See the
[dashboard README](../dashboard/README.md) for the corresponding frontend setup.

The backend requires a reachable Lightning provider. The
[integration-test regtest stack](../tests/itest/README.md) supplies reproducible
LND and CLN nodes; [Polar](https://lightningpolar.com/) is another local-node option.

Before submitting backend changes, run:

```bash
make fmt
make lint
make build
make test
```

Changes to account, wallet, payment, invoice, address, authentication, or
database behavior should also run at least one black-box integration cell:

```bash
make test-integration ITEST_DATABASE=sqlite ITEST_PROVIDER=lnd_grpc
make test-integration-fresh ITEST_DATABASE=sqlite ITEST_PROVIDER=lnd_grpc
```

Dashboard changes should be checked from `dashboard/`:

```bash
yarn lint
yarn typecheck
yarn test
yarn build
yarn fm:check
```

Run `make openapi` after changing backend routes or shared API types. It refreshes
both the checked-in OpenAPI document and the generated dashboard client.

Update the changelog for user-visible behavior changes. Documentation, tests,
and internal refactors alone do not require an entry. For release preparation,
follow [RELEASING.md](RELEASING.md).
