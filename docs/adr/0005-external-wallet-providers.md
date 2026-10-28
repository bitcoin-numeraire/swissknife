# ADR 0005: External LDK Server and Wavelength providers

- Status: Proposed; LDK Lightning adapter and Wavelength RPC client implemented
- Date: 2026-10-28
- Issues: #383, #384, #385, #386, #387, #388, #389, #390

## Provider boundary

LDK Server is an external daemon embedding LDK Node. Wavelength's `waved` is
an external Ark wallet daemon. Both can supply liquidity for the existing
SwissKnife account/wallet ledger. Changing providers never rewrites balances
or introduces a provider-backing limit on authorized credits.

A shared daemon holds installation-level keys; SwissKnife accounts remain
separate internal ledger accounts. This does not introduce independently held
Ark keys for every account. A personal server can avoid managing Lightning
channels with Wavelength, but still depends on an Ark operator, swaps, wallet
refresh and recovery/exit operation. Operator-side Lightning infrastructure
does not imply a Lightning-node requirement on the wallet client.

## LDK implementation

Select `ln_provider = "ldk_server"` and set `[ldk_server_config]` as in
`config/default.toml`. The endpoint is `host:port`, without a scheme. The
certificate must verify that host. LDK macaroon files contain hex strings,
unlike LND's binary files. Use the correct permissions for node information,
invoice creation, outgoing payments and payment-history reads.

The adapter pins upstream protobufs and uses the upstream macaroon library to
bind every unary request to its method, exact gRPC frame and current time.
Clocks must agree within the upstream request-binding window. Tonic provides
the verified Rustls transport. The upstream Rust client's reqwest constructor
does not force Rustls, so Cargo feature unification with SwissKnife's Socket.IO
dependency changes its TLS backend. Using tonic avoids that coupling.

BOLT11 receives preserve millisatoshis, expiry and description hashes. Fee
quotes expose the configured maximum and leave the estimate absent because
LDK Server exposes no route-estimation RPC. Sends enforce that same maximum.
Payment IDs are not assumed to be hashes: bounded, paginated history queries
match the BOLT11 hash and direction. Polling calls existing idempotent replay
use cases, including after startup. Outgoing settlement verifies the preimage
and requires actual fee data. Unknown send outcomes retain their reservation.

Ordinary auto-claim invoice cancellation is unavailable upstream. SwissKnife's
existing best-effort cancellation after internal settlement reports this
limitation. The hold-invoice fail RPC is not a substitute for cancellation of
an ordinary invoice.

LDK's on-chain RPC can send a transaction immediately, but it does not expose
the PSBT preparation, locked inputs, fee quote or output history expected by
`BitcoinWallet`. The initial adapter therefore rejects SwissKnife on-chain
operations explicitly. It never manufactures a PSBT, txid, output or successful
empty history. A later capability-aware chain port can support immediate-send
and asynchronous wallets; existing LND/CLN behavior remains available.

## Wavelength implementation and gates

`crates/wavelength-client` is a standalone transport crate with pinned protobufs,
TLS/macaroon authentication, unary deadlines, live streams and bounded complete
activity pagination. SwissKnife-specific mapping stays outside the crate.
No Wavelength provider is selectable yet, and the default remains unchanged.

The current daemon API requires these decisions/additions before parity:

- **Receive contract:** `RecvRequest` and `StartReceiveRequest` support amount
  and memo, but not a BOLT11 description hash or caller-selected expiry. LNURL
  requires the hash of its metadata. A memo containing the hash is not equivalent.
- **Amounts:** wallet and swap amounts use whole sats. Fractional-msat requests
  must be rejected explicitly; never round. Zero-amount receive is unavailable.
- **Cancellation:** no ordinary receive cancellation RPC exists. Decide how to
  handle internally settled invoices that can still receive externally.
- **Fees:** zero `max_fee_sat` means no caller cap. A zero/sub-sat SwissKnife
  fee budget must not become an unlimited daemon authorization. Complete quotes
  must be distinguished from local-only previews, and credit top-ups must be
  counted in the total outflow. Small receives may use operator credit instead
  of a self-custodial VTXO; this must be an explicit product policy.
- **Recovery:** send intents are volatile and consumed before dispatch. The
  adapter must persist the payment hash before sending, retain uncertain
  outcomes, and reconcile activity without automatically preparing/sending
  again. Credit-backed receive activity IDs differ from payment hashes.
- **On-chain:** boarding and cooperative leave are asynchronous Ark operations,
  not ordinary PSBT funding. Output identity, confirmation, fees and crediting
  points need a distinct mapping/port rather than invented transaction data.
- **TLS interoperability:** at the pinned revision the generated certificate
  has `CA:TRUE`; Rustls rejects it as an end-entity certificate. A supplied
  server certificate works. Fix generation upstream rather than relaxing TLS.
- **Readiness:** `WalletService.Status` calls balance and returns
  `FailedPrecondition` before wallet creation rather than reporting locked/not
  ready status. Bootstrap must distinguish this from bad credentials/outage.

## Verification and default cutover

The LDK Docker topology uses a pinned source/image and a real funded LND/LDK
channel, including LDK's on-chain anchor reserve. Its Lightning-only API suite
shares account/auth/invoice/LNURL tests and asserts unsupported on-chain calls.
The existing four adapters retain their complete on-chain integration suite.

The Wavelength Docker test exercises a real wallet-enabled daemon with verified
TLS and authentication. It is a transport gate, not evidence of funded Ark
payment support. The complete operator/swap/indexer/client topology, refunds,
restarts, offline catch-up and recovery remain #389. Only after #388/#389 and
the compatibility decisions above are resolved should #390 change new-install
defaults and deployment/documentation surfaces.
