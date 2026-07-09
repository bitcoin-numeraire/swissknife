# Contributing

SwissKnife is implemented in Rust with the toolchain pinned in
`rust-toolchain.toml`. The local development infrastructure uses `Docker` and
`Docker Compose`. To run the project locally, run:

```bash
make install-tools
make up
```

Once the DB is up and migrations have executed, run the app:

```
cargo run
```

> If not using a Lightning provider, running dependencies locally can be done easily with [Polar](https://lightningpolar.com/).

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
