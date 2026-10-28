# Changelog

All notable user-facing changes to SwissKnife will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Release entries from this file can be copied into the corresponding GitHub
release notes when a tag is published.

## [Unreleased]

### Fixed

- Restore administrative transaction cleanup across wallets and report empty
  cleanup results and API errors in the dashboard.

## [0.3.0] - 2026-10-28

### Added

- Accounts with authentication identities, preferences, and multiple
  asset-scoped wallets. Administrative account and wallet directories support
  account creation and permission management
  ([#297](https://github.com/bitcoin-numeraire/swissknife/issues/297),
  [#331](https://github.com/bitcoin-numeraire/swissknife/pull/331),
  [#342](https://github.com/bitcoin-numeraire/swissknife/pull/342)).
- Payment fee quotes for Lightning, on-chain, and internal transfers. Lightning
  quotes distinguish the estimated fee from the provider-enforced maximum
  ([#345](https://github.com/bitcoin-numeraire/swissknife/pull/345)).
- Durable account settlement events over server-sent events, with reconnect
  replay and live dashboard balance and transaction updates
  ([#347](https://github.com/bitcoin-numeraire/swissknife/pull/347),
  [#353](https://github.com/bitcoin-numeraire/swissknife/pull/353)).
- LNURL success actions retained through settlement
  ([#346](https://github.com/bitcoin-numeraire/swissknife/pull/346)).
- Password changes for the local JWT owner account.

### Changed

- **Breaking:** `/v1/me` returns an account. Wallet operations use explicit
  `/v1/me/wallets/{wallet_id}/...` routes, and assets and networks are derived
  from the selected wallet. The legacy wallet-as-user API is removed
  ([#319](https://github.com/bitcoin-numeraire/swissknife/pull/319),
  [#321](https://github.com/bitcoin-numeraire/swissknife/pull/321)).
- Redesigned wallet, activity, identity, settings, and administration views,
  with a persisted wallet selector and improved mobile payment and receive flows.
- Core Lightning payments use `xpay`. Absolute provider fee limits replace the
  old `fee_buffer`, `maxfeepercent`, and `payment_exemptfee` settings
  ([#282](https://github.com/bitcoin-numeraire/swissknife/pull/282),
  [#345](https://github.com/bitcoin-numeraire/swissknife/pull/345)).

### Fixed

- Preserve pending payments and their reservations when Lightning sends or
  on-chain broadcasts have an unknown outcome. Dashboard confirmations now
  distinguish pending, settled, and failed payments
  ([#374](https://github.com/bitcoin-numeraire/swissknife/pull/374),
  [#375](https://github.com/bitcoin-numeraire/swissknife/pull/375),
  [#380](https://github.com/bitcoin-numeraire/swissknife/pull/380)).
- Credit recovered CLN invoices and retry deposits after transient database
  failures. LND node self-transfers to registered deposit addresses now credit
  the receiving wallet
  ([#316](https://github.com/bitcoin-numeraire/swissknife/pull/316),
  [#369](https://github.com/bitcoin-numeraire/swissknife/pull/369),
  [#380](https://github.com/bitcoin-numeraire/swissknife/pull/380)).
- Preserve original Lightning payment amounts when completion events report
  partial or different amounts, avoiding premature settlement and release of funds
  ([#376](https://github.com/bitcoin-numeraire/swissknife/pull/376)).
- Correct CLN REST routing-fee accounting and settlement when actual fees exceed
  the reservation
  ([#282](https://github.com/bitcoin-numeraire/swissknife/pull/282),
  [#313](https://github.com/bitcoin-numeraire/swissknife/pull/313)).
- Validate internal BOLT11 payments against the issued invoice's amount and signature
  ([#365](https://github.com/bitcoin-numeraire/swissknife/pull/365)).
- Reject on-chain quotes and sends before preparing a transaction when the wallet
  cannot cover the payment amount; discard stale dashboard fee estimates
  ([#358](https://github.com/bitcoin-numeraire/swissknife/pull/358),
  [#371](https://github.com/bitcoin-numeraire/swissknife/pull/371)).
- Keep receive QR codes consistent with their amount, memo, and expiry. Expired
  Unified requests require explicit on-chain selection before sending
  ([#367](https://github.com/bitcoin-numeraire/swissknife/pull/367),
  [#368](https://github.com/bitcoin-numeraire/swissknife/pull/368)).
- Reject administrative bulk deletes without an effective row-selection filter
  with HTTP 400
  ([#370](https://github.com/bitcoin-numeraire/swissknife/pull/370)).
- Bound external LNURL responses to 256 KiB and each fetch to 10 seconds
  ([#372](https://github.com/bitcoin-numeraire/swissknife/pull/372)).
- Return deterministic conflicts for concurrent local sign-up and reject invoice
  creation through a wallet on the wrong Bitcoin network
  ([#334](https://github.com/bitcoin-numeraire/swissknife/pull/334)).

### Security

- Replace the vulnerable JWT cryptography backend with AWS-LC and remove the
  obsolete RSA audit exception
  ([#376](https://github.com/bitcoin-numeraire/swissknife/pull/376)).
- Update password-hashing, TLS, application dependencies, and container runtimes
  to address security advisories
  ([#322](https://github.com/bitcoin-numeraire/swissknife/pull/322),
  [#324](https://github.com/bitcoin-numeraire/swissknife/pull/324),
  [#363](https://github.com/bitcoin-numeraire/swissknife/pull/363)).

## [0.2.0] - 2026-06-18

### Added

- Added Bitcoin on-chain wallet support for self-hosted CLN and LND setups,
  including address management, wallet synchronization, and transaction
  preparation before broadcast ([#182], [#183], [#184], [#196], [#203], [#204],
  [#209]).
- Added LND gRPC support for Lightning operations ([#205]).
- Added support for base64-encoded Lightning macaroons in provider configuration
  ([#176]).
- Added black-box integration tests for public API flows, regtest LND/CLN
  providers, LNURL, OAuth2/OIDC, and SQLite/Postgres persistence and
  concurrency coverage ([#240], [#267]).

### Changed

- Improved Lightning event-listener startup synchronization so pending invoices,
  payments, and wallet state are reconciled more reliably when SwissKnife starts
  ([#200], [#202]).
- Refreshed the backend dependency baseline ([#210], [#212], [#226]).
- Upgraded the dashboard to React 19, Next.js 16, MUI v9, and zod 4 on Node 24 /
  Yarn 4.17, and refreshed its dependency baseline ([#219], [#276]).
- Aligned the dashboard with the current backend API and regenerated its typed
  client; added a `make openapi` workflow to keep the spec and client in sync
  ([#221]).
- Dropped Breez Liquid/Spark support for now, keeping the current release focused
  on self-hosted CLN and LND providers ([#224]).

### Fixed

- Fixed incorrect balance computation in wallet overviews ([398e89f]).
- Fixed LNURL payment callback encoding and error handling ([#224]).
- Fixed on-chain deposit event handling so transient database write failures do
  not silently drop credits ([#267]).

## [0.1.8] - 2025-11-03

### Fixed

- Removed blink on the login page ([#175]).

[Unreleased]: https://github.com/bitcoin-numeraire/swissknife/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/bitcoin-numeraire/swissknife/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/bitcoin-numeraire/swissknife/compare/v0.1.8...v0.2.0
[0.1.8]: https://github.com/bitcoin-numeraire/swissknife/releases/tag/v0.1.8
[#175]: https://github.com/bitcoin-numeraire/swissknife/pull/175
[#176]: https://github.com/bitcoin-numeraire/swissknife/pull/176
[#182]: https://github.com/bitcoin-numeraire/swissknife/pull/182
[#183]: https://github.com/bitcoin-numeraire/swissknife/pull/183
[#184]: https://github.com/bitcoin-numeraire/swissknife/pull/184
[#196]: https://github.com/bitcoin-numeraire/swissknife/pull/196
[#200]: https://github.com/bitcoin-numeraire/swissknife/pull/200
[#202]: https://github.com/bitcoin-numeraire/swissknife/pull/202
[#203]: https://github.com/bitcoin-numeraire/swissknife/pull/203
[#204]: https://github.com/bitcoin-numeraire/swissknife/pull/204
[#205]: https://github.com/bitcoin-numeraire/swissknife/pull/205
[#209]: https://github.com/bitcoin-numeraire/swissknife/pull/209
[#210]: https://github.com/bitcoin-numeraire/swissknife/pull/210
[#212]: https://github.com/bitcoin-numeraire/swissknife/pull/212
[#219]: https://github.com/bitcoin-numeraire/swissknife/issues/219
[#221]: https://github.com/bitcoin-numeraire/swissknife/issues/221
[#224]: https://github.com/bitcoin-numeraire/swissknife/pull/224
[#226]: https://github.com/bitcoin-numeraire/swissknife/pull/226
[#240]: https://github.com/bitcoin-numeraire/swissknife/issues/240
[#267]: https://github.com/bitcoin-numeraire/swissknife/issues/267
[#276]: https://github.com/bitcoin-numeraire/swissknife/pull/276
[#282]: https://github.com/bitcoin-numeraire/swissknife/pull/282
[#297]: https://github.com/bitcoin-numeraire/swissknife/issues/297
[#319]: https://github.com/bitcoin-numeraire/swissknife/pull/319
[#321]: https://github.com/bitcoin-numeraire/swissknife/pull/321
[#324]: https://github.com/bitcoin-numeraire/swissknife/pull/324
[#331]: https://github.com/bitcoin-numeraire/swissknife/pull/331
[398e89f]: https://github.com/bitcoin-numeraire/swissknife/commit/398e89f
