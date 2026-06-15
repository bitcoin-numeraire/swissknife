//! Integration tests for the SwissKnife backend.
//!
//! Each test talks to a real, running SwissKnife instance through its public
//! HTTP API only — never through internal services or repositories. The
//! instance is the actual compiled binary, spawned by the harness against the
//! dockerized regtest stack (bitcoind + LND + CLN + Postgres + a mock OpenID
//! provider), with an in-process `wiremock` server standing in for external
//! LNURL services. See `tests/itest/README.md` for how to bring up the
//! dependencies.
//!
//! Layout:
//!   common/   the harness — process spawn, HTTP client, fixtures, assertions
//!   suites/   one module per API domain; tests are grouped and named like the
//!             unit tests (`mod <endpoint> { fn <case> }`)
//!
//! The whole suite is gated behind the `itest` feature so a plain `cargo test`
//! runs only unit tests; run it with `cargo test --features itest --test api`
//! (or `make test-integration`).
#![cfg(feature = "itest")]

mod common;
mod suites;
