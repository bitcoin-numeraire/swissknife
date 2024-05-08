# Numeraire Bitcoin SwissKnife

Numeraire's Bitcoin SwissKnife is an orchestator application made of several components and a frontend application enabling easy integration of Bitcoin, the Lightning network and the RGB protocol to any entity or organization that wishes to do so without handling the complexity of the above technologies.

## Account and Key management (WIP)

SwissKnife enables account and key management through its `Wallet` infrastructure. By securing private keys in secure HSMs, Numeraire SwissKnife is able to completely isolate the private keys from the outside world, enabling cryptographic operations to be done in isolation. No private keys are ever stored outside of a secure, specialised HSM server or hardware wallet.

The compatible HSMs are:

- [] [`Azure Key Vault`](https://azure.microsoft.com/en-us/products/key-vault)
- [] [`AWS KMS`](https://aws.amazon.com/kms/)
- [] [`Hashicorp Vault`](https://www.vaultproject.io/) (with the use of a custom plugin)
- [] Bare-metal HSMs (to be decided)
- [] Cold storage hardware wallets by exporting and importing `PSBTs`

## Lightning Integration

### Providers

Numeraire SwissKnife allows Lightning integration via integration with well-known providers.

The compatible providers are:

- [x] [`Greenlight`](https://blockstream.com/lightning/greenlight/) (mainly used in conjunction with `Breez`)
- [x] [`Breez SDK`](https://breez.technology/sdk/) (allowing switching between LSPs.). Please contact us to get your API key.
- [] [`LightSpark`](https://www.lightspark.com/)
- [] Direct [`Core Lightning`](https://corelightning.org/) integration. (WIP)
- [] Direct [`LND`](https://github.com/lightningnetwork/lnd) Integration

### Lightning Address

Numeraire SwissKnife allows any entity or organization to create its own Lightning Address infrastructure. Like an email provider, any company can deploy Lightning Addresses (`username@your.domain`) to send and receive Lightning payments.

> This functionality needs to be coupled with a Provider for liquidity purposes.

Lightning addresses can then be used to create complex flows by triggering webhooks or performing specific actions upon payment reception on RGB smart contracts.

## RGB protocol and Taproot Assets

### Assets issuance

Numeraire SwissKnife enables any entity to become a smart contract issuer on the RGB and Taproot Assets protocols by deploying smart contracts on the Bitcoin Blockchain (Lightning integration to come).

Multiple use cases are possible using RGB and Taproot Assets:

- [] Asset tokenization and real-world assets (RGB-21/UDA, aka Unique Digital Assets)
- [] Currencies and stablecoins (RGB-20/NIA, Non Inflatable Assets)
- [] Collectible collections (RGB-25/CFA aka Collectible Fungible Assets)

### Assets data storage and encryption

Numeraire SwissKnife allows for the encrypted storage of the smart contract metadata. Tokenized assets such as art NFTs or contract terms can be stored, retrieved and distributed using Swissknife's API.

### Contract propagation

Because RGB does not store the smart contract on-chain like Ethereum or other smart contract protocols. Contracts can be sent to other parties confidentially through other means. Numeraire SwissKnife allors contract propagation through a JSON-RPC proxy server implementation. With the following techologies to be implemented eventually:

- [] `JSON-RPC proxy server`
- [] `Filecoin`
- [] `IPFS`
- [] `Email`
- [] `Taproot Assets Universe`

## Authentication and RBAC

NumeraireSwissknife allows full authentication, account segregation and authorization:

- [x] `JWKS server with automatic public key retrieval`
- [x] `JWT token authentication`
- [x] `RBAC per route`
- [] `API keys authentication`
