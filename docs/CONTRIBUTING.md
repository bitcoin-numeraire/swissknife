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
