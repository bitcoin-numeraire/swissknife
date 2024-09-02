<div align="center">
  <img src="https://numeraire.fra1.cdn.digitaloceanspaces.com/development/LOGO_FULL_PNG_color_dark_9d58514132.png" height="200"/>
  <h1>SwissKnife</h1>
  <h3>A self-custodial wallet & transaction orchestrator for Lightning, Nostr and Smart contract protocols on Bitcoin</h3>
</div>

# SwissKnife

#### Numeraire SwissKnife is a wallet and transaction orchestrator enabling easy integration of Bitcoin, Lightning, Nostr and smart contract protocols to any entity that wishes so without the complexities of above technologies.

[![GitHub stars](https://img.shields.io/github/stars/bitcoin-numeraire/swissknife.svg?style=social&label=Star&maxAge=1)](https://github.com/bitcoin-numeraire/swissknife/stargazers)
If you like what we do, consider starring, sharing and contributing!

> Check the documentation [here](https://docs.numeraire.tech/swissknife).

## Features

- [`Lightning Network`](https://github.com/lnurl/luds). Send and receive payments.
- [`LNURL`](https://github.com/lnurl/luds) support.
- [`Lightning Address`](https://lightningaddress.com/). Deploy your own Lightning Address infrastructure. Like email, anyone can use identifiers (`username@your.domain`) to send and receive payments.
- Generate invoices.
- Account segregation. Support any amount of users.
- Internal ledger for instant payments on the same SwissKnife instance.
- REST API.
- Frequent contacts.
- `JWKS` with automatic public key retrieval
- `JWT` token authentication` (tested with Auth0, Supabase).
- `RBAC`. Fine grained authorization per route.
- Data availability through pagination and advanced search.

Numeraire SwissKnife ships with a [Dashboard](https://github.com/bitcoin-numeraire/swissknife-dashboard).

## Lightning Providers

SwissKnife allows direct Lightning integration, supporting the most used node implementations and well-known providers:

- [`Core Lightning`](https://corelightning.org/):
  - Non-custodial
  - Run your own node
  - Manage your own liquidity.
- [`Greenlight`](https://blockstream.com/lightning/greenlight/):
  - Non-custodial
  - Automatic node management.
  - Manage your own liquidity.
- [`Breez`](https://breez.technology/sdk/):
  - Non-custodial
  - Automatic node management.
  - Automatic liquidity management via LSPs (user can switch LSPs)

## Documentation

Extended documentation is available [here](https://docs.numeraire.tech/swissknife)

## Work In Progress (WIP)

#### Features

- [ ] [`Nostr`](https://github.com/nostr-protocol/nips/blob/master/57.md). NIP-5 and Zap support through Lightning Address
- [ ] Webhooks
- [ ] API keys authentication
- [ ] BOLT12 (offers)
- [ ] Notifications (Email, SMS by Twilio)
- [ ] Documentation website
- [ ] Dockerhub images
- [ ] Desktop applications
- [ ] Helm Charts

#### Lightning providers

- [ ] [`Phoenixd`](https://phoenix.acinq.co/server)
  - Non-custodial
  - Automatic node management
  - Automatic liquidity management via ACINQ.
- [ ] [`LightSpark`](https://www.lightspark.com/)
  - Custodial
  - Automatic node management
  - Automatic liquidity management via Lightspark
- [ ] [`LND`](https://github.com/lightningnetwork/lnd)
  - Non-custodial
  - Run your own node
  - Manage your own liquidity

#### Smart contracts

- [x] [RGB](https://rgb.tech/) Smart contracts
- [ ] [Taproot Assets](https://docs.lightning.engineering/the-lightning-network/taproot-assets).
