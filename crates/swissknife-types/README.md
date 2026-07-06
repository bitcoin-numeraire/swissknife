# swissknife-types

Shared API/contract types for [Numeraire SwissKnife](https://github.com/bitcoin-numeraire/swissknife).

This crate is the single source of truth for the shapes that cross SwissKnife's
boundary. The backend serializes them as HTTP responses, decodes requests into
them, and derives its OpenAPI schema from them; the integration tests and any
generated client/SDK consume the same types.

## What's inside

- **Entities** — `Account`, `AuthIdentity`, `AccountPreferences`, `Invoice`,
  `Payment`, `Wallet`, `LnAddress`, `BtcAddress`, `BtcOutput`, `ApiKey`, … the
  core records. They serialize directly as the wire representation; there are no
  parallel `*Response` types.
- **Requests** — `SendPaymentRequest`, `NewInvoiceRequest`, … the inputs decoded
  from request bodies and query strings.
- **Responses** — the few edge shapes with no entity: `SignInResponse`,
  `NostrNIP05Response`, `ErrorResponse`, `LnUrlCallback`.
- **Shared enums** — `AuthProvider`, `Currency`, `Ledger`, `Permission`,
  `BtcNetwork`, `InvoiceStatus`, `PaymentStatus`, `BtcAddressType`, …

## Principles

- **Pure data.** The types carry no behaviour — validation and business rules
  live in the application's use cases, never on the types here.
- **The wire shape lives with the type.** Each type carries its own `serde` and
  `utoipa::ToSchema` annotations, so what you read is what's serialized.
- **Sensitive internal-only fields are `#[serde(skip)]`** so they never reach the
  wire (e.g. `ApiKey::key_hash`).

## Usage

```toml
[dependencies]
swissknife-types = { path = "crates/swissknife-types" }
```

```rust
use swissknife_types::{Invoice, SendPaymentRequest};
```
