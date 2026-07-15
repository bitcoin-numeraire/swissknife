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

Polar's LND certificate is marked as a certificate authority and is therefore
rejected as a server certificate by rustls. When using LND over gRPC with
Polar, point `lnd_grpc_config.cert_path` at the node's `tls.cert` and enable
exact certificate pinning:

```toml
[lnd_grpc_config]
endpoint = "https://127.0.0.1:10001"
cert_path = "/path/to/polar/node/tls.cert"
pin_server_certificate = true
```

Pinning is intended for local development only. It accepts only the configured
certificate and still verifies that the server owns its private key. Leave it
disabled when `cert_path` contains a proper CA certificate, including the
integration-test LND network.

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
