# Changelog

All notable user-facing changes to SwissKnife will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Release entries from this file can be copied into the corresponding GitHub
release notes when a tag is published.

## [Unreleased]

### Added

- Added explicit accounts, authentication identities, account preferences, and
  an asset catalog. Accounts can own multiple wallets, with each wallet holding
  exactly one asset on one settlement network ([#297]).
- Added account-shaped `/v1/me` APIs and wallet-scoped authenticated routes for
  balances, payments, invoices, Bitcoin addresses, contacts, and API keys
  ([#319]).
- Added administrative account CRUD. Accounts created by administrators do not
  implicitly create a login identity ([#331]).
- Added an authenticated dashboard account context and header wallet selector;
  wallet-scoped pages now use an explicit persisted wallet selection instead of
  inferring the first wallet ([#326]).

### Changed

- Replaced the legacy wallet-as-user contract with account-owned, asset-scoped
  wallets. Payments and invoices now derive their asset and network from the
  selected wallet, without legacy currency fallbacks ([#314], [#315], [#317],
  [#318], [#320], [#321]).

- Migrated CLN Lightning payments from the deprecated `pay` RPC to `xpay`, and
  refreshed the vendored CLN and LND gRPC protos to CLN v26.06 and LND v0.21
  ([#282]).
- Replaced the CLN `maxfeepercent` and `payment_exemptfee` settings with a single
  absolute `maxfee` (msat); when left unset, the node applies its own default
  ([#282]).
- Upgraded backend and dashboard dependencies to their latest compatible
  versions, including `lnurl-rs`, `tower-http`, and `mockall` on the backend and
  the MUI, Next.js, and toolchain packages on the dashboard ([#324]).

### Fixed

- Fixed CLN REST Lightning payments charging the routing fee twice for payments
  that incur a non-zero fee ([#282]).

### Security

- Updated `bcrypt` to 0.19.2, fixing a panic in `bcrypt::verify` on non-ASCII
  hash input (RUSTSEC-2026-0199), and moved the `bitcoin` crate off the yanked
  0.32.100 release ([#324]).

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

[Unreleased]: https://github.com/bitcoin-numeraire/swissknife/compare/v0.2.0...HEAD
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
[#314]: https://github.com/bitcoin-numeraire/swissknife/pull/314
[#315]: https://github.com/bitcoin-numeraire/swissknife/pull/315
[#317]: https://github.com/bitcoin-numeraire/swissknife/pull/317
[#318]: https://github.com/bitcoin-numeraire/swissknife/pull/318
[#319]: https://github.com/bitcoin-numeraire/swissknife/pull/319
[#320]: https://github.com/bitcoin-numeraire/swissknife/pull/320
[#321]: https://github.com/bitcoin-numeraire/swissknife/pull/321
[#324]: https://github.com/bitcoin-numeraire/swissknife/pull/324
[#326]: https://github.com/bitcoin-numeraire/swissknife/issues/326
[#331]: https://github.com/bitcoin-numeraire/swissknife/pull/331
[398e89f]: https://github.com/bitcoin-numeraire/swissknife/commit/398e89f
