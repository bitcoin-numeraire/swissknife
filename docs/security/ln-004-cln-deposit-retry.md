# LN-004: preserve the CLN deposit cursor on output lookup failure

## Verified behavior and scope

Baseline: `b288918c86661d48f2579b3235ef23cbbf258b20`.

The CLN gRPC listener's `handle_chainmoves` logs and skips a deposit when either the initial unspent-output lookup or its spent-inclusive fallback returns an error. `listen_chainmoves` then checkpoints the end of the successful-looking batch. Reconnecting starts from that checkpoint, so an unresolved deposit before it is no longer replayed by this listener.

The original report overstates the scope: both CLN gRPC and REST `BitcoinWallet::synchronize` implementations already use one spent-inclusive `listfunds` call and propagate its error before returning any batch/cursor. They do not contain the reported per-output error-swallowing path. Do not change those clients for this fix.

## Invariant and bounded correction

A failed wallet lookup is not a successful deposit projection. Return `LightningError::Listener` from `handle_chainmoves` on lookup failure, leaving the persisted cursor unchanged. The existing listener supervisor reconnects with backoff and replays from that cursor. Preserve the bounded unspent-first lookup, spent-inclusive fallback on `Ok(None)`, current unknown-output behavior, and propagation of projection/write errors.

Only the gRPC listener error branch changes. No schema, dependency, public API, accounting, transport policy, wallet reservation, or provider-payment changes are required. SQLite and PostgreSQL continue to use the same cursor storage and idempotent deposit projection.

## Test plan

- Before implementation, run a deterministic `itest`-gated adapter component regression with a loopback CLN gRPC fixture and generated wallet/use-case mocks. An output-lookup error must leave the checkpoint unchanged; retry must list from the original index and project the deposit before advancing it.
- Cover both the initial lookup and spent-inclusive fallback error, successful unspent lookup, successful spent-output fallback, missing output, and event-projection failure. Assert exact outpoints, fallback flags, cursor writes and original replay index.
- The loopback fixture implements only `Wait` and `ListChainMoves`; it is not CLN or a full database/API integration test. No real-money transactions or public-service testing.
- Run workspace unit tests before/after, focused component tests, formatter, strict clippy, build and pre-commit diff/static review. Existing database-backed idempotency tests provide separate replay coverage where executed.

## Trade-offs and limitations

A persistent lookup failure stalls the batch instead of skipping accounting work; this is intentional and uses the existing reconnect backoff. Previously checkpointed missed deposits are not automatically repaired or rewound. Operators who suspect past omissions need an explicit reconciliation plan; this PR does not mutate historical cursors. `Ok(None)` retains existing skip behavior, rather than conflating an absent output with a transport failure. Multi-replica cursor serialization, confirmation/reorg policy and aggregate outgoing-payment semantics are separate reported work.

## Exact-version primary references

Deployment/test baseline pins Core Lightning `v26.06.9`. Its RPC schemas specify that `listchainmoves` returns confirmed on-chain balance changes, `created_index` is a 1-based creation-order index, `start` selects the replay starting value, and `listfunds(spent=true)` includes spent outputs:

- https://github.com/ElementsProject/lightning/blob/v26.06.9/doc/schemas/listchainmoves.json
- https://github.com/ElementsProject/lightning/blob/v26.06.9/doc/schemas/listfunds.json
- https://github.com/ElementsProject/lightning/blob/v26.06.9/doc/schemas/wait.json

The fixture follows the installed `tonic 0.14.6` server `UnaryService` / `NamedService` contracts and `tonic-prost 0.14.6` codec; it uses the repository's generated protobuf request/response types, not handwritten wire payloads.
