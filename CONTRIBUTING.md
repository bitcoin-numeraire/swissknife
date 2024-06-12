# Contributing

Swissknife is implemetend in Rust. The local development infrastructure uses `Docker`and `Docker Compose`. To run the project locally, simply run:

```bash
make install-tools
make up
```

Once the DB is up and migrations have executed, run the app:

```
cargo run
```

> If not using a Lightning provider, running dependencies locally can be done easily with [Polar](https://lightningpolar.com/).
