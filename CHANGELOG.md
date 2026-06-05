# Changelog

All notable user-facing changes to SwissKnife will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Release entries from this file can be copied into the corresponding GitHub
release notes when a tag is published.

## [Unreleased]

### Added

- Added Bitcoin on-chain wallet support for self-hosted CLN and LND setups,
  including address management, wallet synchronization, and transaction
  preparation before broadcast ([#182], [#183], [#184], [#196], [#203], [#204],
  [#209]).
- Added LND gRPC support for Lightning operations ([#205]).
- Added support for base64-encoded Lightning macaroons in provider configuration
  ([#176]).

### Changed

- Improved Lightning event-listener startup synchronization so pending invoices,
  payments, and wallet state are reconciled more reliably when SwissKnife starts
  ([#200], [#202]).
- Refreshed the backend dependency baseline ([#210], [#212], [#226]).

### Removed

- Removed Breez Liquid/Spark support for now, keeping the current release focused
  on self-hosted CLN and LND providers ([#224]).

### Fixed

- Fixed incorrect balance computation in wallet overviews ([398e89f]).
- Fixed LNURL payment callback encoding and error handling ([#224]).

## [0.1.8] - 2025-11-03

### Fixed

- Removed blink on the login page ([#175]).

[Unreleased]: https://github.com/bitcoin-numeraire/swissknife/compare/v0.1.8...HEAD
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
[#224]: https://github.com/bitcoin-numeraire/swissknife/pull/224
[#226]: https://github.com/bitcoin-numeraire/swissknife/pull/226
[398e89f]: https://github.com/bitcoin-numeraire/swissknife/commit/398e89f
