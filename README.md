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
- [`Nostr`](https://github.com/nostr-protocol/nips/blob/master/05.md). NIP-05 and Zap support through your Lightning Address.
- Generate invoices.
- Account segregation. Support any amount of users.
- Internal ledger for instant payments on the same SwissKnife instance.
- REST API.
- Frequent contacts.
- `JWKS` with automatic public key retrieval
- `JWT` token authentication` (tested with Auth0, Supabase).
- `RBAC`. Fine grained authorization per route.
- Data availability through pagination and advanced search.
- API keys authentication

Numeraire SwissKnife ships with a [Dashboard](https://github.com/bitcoin-numeraire/swissknife/dashboard).

## Lightning Providers

SwissKnife allows direct Lightning integration, supporting the most used node implementations and well-known providers:

- [`LND`](https://github.com/lightningnetwork/lnd)
  - Run your own node
  - Manage your own liquidity
- [`Core Lightning`](https://corelightning.org/):
  - Run your own node
  - Manage your own liquidity.
- [`Breez SDK (Liquid)`](https://breez.technology):
  - Nodeless integration via Breez Liquid SDK
  - Lightning send/receive and Bitcoin swaps supported
  - Note: direct Bitcoin address generation and raw PSBT flows are not exposed by this provider

## Installation

SwissKnife provides multiple Docker deployment options to suit different infrastructure needs:

### Self-Contained Installation (Recommended)

The all-in-one image bundles the backend API and dashboard in a single container:

```bash
docker pull bitcoinnumeraire/swissknife:latest
docker run -p 3000:3000 bitcoinnumeraire/swissknife:latest
```

This image includes:
- Rust backend API server
- Next.js dashboard (static export served by the backend)
- Default configuration at `/config/default.toml`

### Separated Backend and Frontend

For Kubernetes or microservices deployments, use the separated images:

#### Backend Only
```bash
docker pull bitcoinnumeraire/swissknife-server:latest
docker run -p 3000:3000 bitcoinnumeraire/swissknife-server:latest
```

#### Frontend Only (Standalone Next.js Server)
```bash
docker pull bitcoinnumeraire/swissknife-dashboard:latest
docker run -p 8080:8080 bitcoinnumeraire/swissknife-dashboard:latest
```

When using separated deployment:
- Configure the dashboard to point to your backend API endpoint
- Backend runs without the dashboard (`SWISSKNIFE_DASHBOARD_DIR=""`)
- Frontend runs as a standalone Node.js server on port 8080

All images support both `linux/amd64` and `linux/arm64` architectures.

## Documentation

Extended documentation is available [here](https://docs.numeraire.tech/swissknife)

## Work In Progress (WIP)

#### Features

- [ ] Webhooks
- [ ] BOLT12 (offers)
- [ ] Notifications (Email, SMS by Twilio)
- [ ] Desktop applications

#### Lightning providers

- [x] [`Breez SDK (Liquid)`](https://breez.technology)
  - Nodeless
- [ ] [`Breez SDK (Spark)`](https://www.lightspark.com/)
  - Nodeless

#### Smart contracts

- [ ] [RGB](https://rgb.tech/) Smart contracts
- [x] [Taproot Assets](https://docs.lightning.engineering/the-lightning-network/taproot-assets).
