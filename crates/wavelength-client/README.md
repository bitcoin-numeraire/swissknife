# wavelength-client

Async Rust access to an external `waved`, using the wallet and inspection RPC
contract pinned in `proto/README.md`. This crate has no SwissKnife dependency
and does not embed Go, TypeScript, a daemon or an Ark implementation.

`Client::connect(Config)` requires an HTTPS endpoint, PEM daemon certificate,
binary macaroon and positive connect/request timeouts. Credential-bearing
configuration deliberately has no Debug implementation. The client is cloneable
and shares its HTTP/2 channel. All wallet lifecycle, receive, prepare/send,
balance, history, inspection, stream and exit methods use upstream wire types.
Public daemon-owned swap reads expose `QuotePay` without allocating a prepared
intent, `GetSwap` by stable payment hash, and the server-authoritative credit
snapshot. `ListCredits` has no cursor; an absent operation is not evidence of
failure. These APIs use the same verified transport, authentication and deadlines.

Unary calls have local deadlines. Streaming has an establishment deadline and
continues past the unary timeout. No RPC is retried automatically. A failed
`Send` response does not mean the payment failed: reconcile durable activity
using the invoice payment hash before considering another dispatch. Prepared
intents in the pinned daemon are one-use and in memory.

`activity(page_size, max_pages)` reads the entire paginated activity snapshot
or returns an error if pagination is incomplete. Credit-backed receive IDs are
not payment hashes; correlate `request`/`progress` data. `exact_sats` converts
millisatoshis only when divisible by 1000.

Run `cargo test -p wavelength-client` for transport-contract tests. From the
repository root, `tests/itest/scripts/test-wavelength-client.sh` builds the
pinned wallet-enabled Docker image and checks authenticated and unauthenticated
RPCs against a fresh, uninitialized daemon. It does **not** validate funded
Ark/swap payment behavior. Funded public-network coverage is tracked in
SwissKnife #389.

`tests/itest/scripts/test-wavelength-public.sh` is a separate, opt-in test
against documented public signet services (`WAVELENGTH_PUBLIC_NETWORK=testnet`
selects testnet3). It creates an ephemeral unfunded wallet and verifies boarding
address creation, an unpaid receive invoice, restart/unlock and receive-history
persistence. Only public `waved` runs in Docker; no private backend is needed.
This verifies external interoperability, not funded settlement or recovery.

The pinned daemon's generated TLS certificate has `CA:TRUE`, which Rustls
rejects as an end-entity certificate. Supply a proper server certificate; the
Docker test uses a publicly known, test-only certificate/key from `tests/fixtures`.
Certificate verification is never disabled. `Status` on an uninitialized
daemon currently returns `FailedPrecondition` rather than a readiness object.
