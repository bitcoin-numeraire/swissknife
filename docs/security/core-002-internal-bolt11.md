# Bind internal settlement to the issued BOLT11

- Audit finding: CORE-002 (pre-production)
- Baseline: `b288918c86661d48f2579b3235ef23cbbf258b20`
- Scope: internal BOLT11 sends and fee quotes only; no schema or provider lifecycle changes.

## Verified cause and threat model

`PaymentService::send_bolt11` selects a receiver invoice by payment hash and uses the submitted invoice amount without authenticating the submitted instrument against the issued one. `estimate_fee` makes the same hash-only classification. The payment hash is public and is not the invoice signing digest. An authenticated account owning a funded, network-compatible wallet can submit a valid invoice signed with a different key and carrying the same hash. The signature parser validates that submitted invoice, not its relationship to the locally issued invoice.

The self-service route is `POST /v1/me/wallets/{wallet_id}/payments`, with account ownership checked in `wallet_pay`; quotes use its `/fee-estimate` sibling. Administrative `/v1/payments` routes additionally require `write:transaction`. The exploit does not require administrative permissions. The impact is a false paid status/event and possible merchant underpayment, not creation of ledger balance out of nothing.

## Proposal and invariants

Before any internal mutation or zero-fee quote, parse the stored and submitted BOLT11 using the locked `lightning-invoice` library and require equality of the parsed signed invoices. This binds all signed fields and the signature to the instrument actually issued. Parsing normalizes valid upper/lowercase representations; a `lightning:` prefix is already removed by the normal input parser. A malformed/missing stored instrument fails closed as an internal inconsistency rather than falling through to an external payment.

Also enforce the stored invoice's fixed amount when present. An amountless issued invoice still accepts a positive caller-supplied amount. Keep existing fixed-invoice precedence for an optional request amount: this bounded change does not introduce a new global amount-override policy. Existing status, self-payment, network and transactional settlement guards remain in force. Apply the same authentication and amount checks in both send and quote paths.

Rejections must not reserve or debit funds, mark the invoice paid, emit settlement events, call a provider payment RPC, or cancel the issued invoice. Genuine issued requests must still settle and quote internally at zero fee. External requests with no local hash match remain unchanged.

## Alternatives rejected

- Signature-parser replacement: the current parser already checks signatures; that is not the missing binding.
- Hash equality alone or hash plus amount: an attacker can copy either public value.
- Payee key only: weaker than binding the full issued request and permits altered signed fields if the issuer reuses a hash.
- String equality: unnecessarily rejects equivalent upper/lowercase encodings.
- Changing invoice cancellation order or implementing issue #355 here: unrelated crash/recovery risks and too broad for this fix.

## Migration and rollback

No migration, new dependency, API shape, or configuration changes. Historical rows without valid stored BOLT11 data will reject internal payment/quote rather than accept a hash-only match; operators should reconcile such rows. Rolling back restores the vulnerable behavior and is not a safe production mitigation.

## Test plan

1. Black-box regtest regression: issue a genuine receiver invoice, sign an attacker-controlled invoice carrying its hash and a smaller amount, submit through an account-owned wallet and expect 422. Verify receiver pending status and unchanged balances/payment history; then settle the genuine invoice successfully.
2. Quote rejection for the forged instrument, without provider dispatch.
3. Domain tests for copied hash, changed signer/amount, missing or invalid stored metadata, fixed stored amount mismatch, amountless genuine requests, uppercase-equivalent genuine requests, and existing status/self-payment guards.
4. Run the regression before implementation and record the expected failure. Run targeted and workspace tests after the fix; format and clippy checks; exercise regtest provider/database cells where available. Disclose unexecuted cells instead of claiming coverage.

## Exact-version library evidence

`Cargo.lock` pins `lightning-invoice` 0.34.1. Its `Bolt11Invoice` and `SignedRawBolt11Invoice` derive `Eq` over the signed raw invoice, cached signing digest and signature (`src/lib.rs:268-270,321-336`). `from_signed` checks field counts, features, signature and amount (`src/lib.rs:1432-1441`). The primary API reference is https://docs.rs/lightning-invoice/0.34.1/lightning_invoice/struct.Bolt11Invoice.html. This fix uses that existing equality/validation API rather than implementing signature verification or interpreting provider cancellation semantics.

## Verification results

The three unit regression slices were executed RED before their respective corrections: a different issued request was accepted by send, the same mismatch received an internal fee quote, and inconsistent stored fixed-amount metadata was ignored. After the correction, workspace tests passed (283 passed, 1 existing ignored test).

The public-API regression passed in all eight local regtest cells: SQLite and PostgreSQL, each with LND gRPC/REST and CLN gRPC/REST. It verifies rejected sends and quotes leave the receiver pending, balances/reservations and payer history unchanged, then settles the genuine uppercase `lightning:`-prefixed request for the full amount with zero fee. This is focused regression coverage, not a run of the entire integration suite.

The pinned mock OAuth2 image cannot start on the local CPU (`CPU does not support x86-64-v2`). Local-JWT/API-key tests used the unchanged regtest setup except that unrelated OAuth2 startup/readiness was omitted. No OAuth2 integration coverage is claimed.

- Workspace tests: `cargo test --locked --workspace --all-targets` — 283 passed, 1 existing ignored OpenAPI generation helper.
- Lint: `cargo clippy --locked --workspace --all-targets --features itest --no-deps -- -D warnings` — passed.
- Formatting: `cargo fmt --all -- --check` — passed.
- Build: `cargo build --locked --workspace --all-targets --features itest` — passed.

Provider timeout/event semantics, invoice cancellation limitations and issue #355 are outside this fix. No live deployments, public-service tests or real-money transactions were used.
