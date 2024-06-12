# Numeraire Bitcoin SwissKnife

Numeraire's Bitcoin SwissKnife is a wallet and transaction orchestrator enabling easy integration of Bitcoin, the Lightning network and smart contract protocols (RGB, Taproot Assets) to any entity or organization that wishes to do so without handling the complexity of the above technologies.

## Features

- [x] [`Lightning Network`](https://github.com/lnurl/luds). Send and receive payments on Clearnet and Tor.
- [x] [`LNURL`](https://github.com/lnurl/luds). Support for the LNURL protocol.
- [x] [`Lightning Address`](https://lightningaddress.com/). Deploy your own Lightning Address infrastructure. Like an email address, anyone can use Lightning addresses (`username@your.domain`) to send and receive Lightning payments.
- [ ] [`Nostr`](https://github.com/nostr-protocol/nips/blob/master/57.md). Zap support through Lightning Address. (TODO).
- [x] Generate invoices.
- [x] Account segregation. Support any amount of users.
- [x] Internal ledger for instant payments on the same SwissKnife instance.
- [x] Public and Admin REST API.
- [ ] Frequent contacts. (TODO)
- [ ] Notifications via webhooks. (TODO)
- [x] `JWKS server with automatic public key retrieval
- [x] `JWT` token authentication` (tested with Auth0).
- [x] `RBAC`. Fine grained authorization per route.
- [ ] API keys authentication. (TODO)
- [x] Horizontal scaling.
- [x] Data availability through pagination and advanced search.
- [ ] [RGB](https://rgb.tech/) Smart contracts. (WIP)
- [ ] [Taproot Assets](https://docs.lightning.engineering/the-lightning-network/taproot-assets). (TODO)

Numeraire SwissKnife ships with a [Dashboard (for admin and users)](https://github.com/bitcoin-numeraire/swissknife-dashboard).

## Lightning Integration

Numeraire SwissKnife allows direct Lightning Network integration, supporting the most used node implementations and well-known providers:

- [x] [`Core Lightning`](https://corelightning.org/):
  - Non-custodial
  - Run your own node
  - Manage your own liquidity.
- [ ] Direct [`LND`](https://github.com/lightningnetwork/lnd) (TODO):
  - Non-custodial
  - Run your own node
  - Manage your own liquidity.
- [x] [`Greenlight`](https://blockstream.com/lightning/greenlight/):
  - Non-custodial
  - Automatic node management.
  - Manage your own liquidity.
- [x] [`Breez`](https://breez.technology/sdk/):
  - Non-custodial
  - Automatic node management.
  - Automatic liquidity management via LSPs (user can switch LSPs)
- [ ] [`Phoenixd`](https://phoenix.acinq.co/server). (TODO):
  - Non-custodial
  - Automatic node management.
  - Automatic liquidity management via ACINQ.
- [ ] [`LightSpark`](https://www.lightspark.com/). (TODO):
  - Custodial
  - Automatic node management.
  - Automatic liquidity management via ACINQ.

# Deployment

Numeraire SwissKnife can be built from source (see Contributing), Docker images and Helm charts will come when the first alpha version is out.

Default configuration is defined in `config/default.toml`. SwissKnife supports `.toml`, `yaml` and `json` config files. The order of applied configuration is the following:

1. ENV vars. Defined given the names of the config values in `default.toml`, adding the prefix `SWISSKNIFE`. Overriding all sensitive values with ENV vars is recommended.
2. any file under `config` corresponding to the `RUN_MODE` (`development` by default). Such as `development.toml|yaml|json` or `production.toml|yaml|json``
3. The `default.toml|yaml|json` file.

Inspect the `.env.example` file for and generate your own `.env` for sensitive config values.
