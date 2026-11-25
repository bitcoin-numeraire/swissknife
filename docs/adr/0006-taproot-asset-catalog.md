# ADR 0006 — Taproot Assets catalog admission

| Field | Value |
| --- | --- |
| Status | Proposed |
| Date | 2026-11-25 |
| Related | [Issue 339](https://github.com/bitcoin-numeraire/swissknife/issues/339), [ADR 0002](0002-identity-assets-wallet-model.md) |

## Decision

SwissKnife keeps a local catalog of assets approved for an installation. A configured Taproot Assets provider supplies verified issuance facts. An operator chooses which issuances the installation supports. This decision applies to an exact issuance ID, including when several issuances share an issuer group key.

| Catalog field | Authority and update rule |
| --- | --- |
| Internal asset UUID | SwissKnife assigns the UUID on first admission. Wallets and historical transactions continue to reference it. |
| Protocol, network, asset reference | Use `taproot_assets`, the Bitcoin network independently confirmed by the configured node, and the verified 32-byte genesis asset ID encoded as 64 lowercase hexadecimal characters. Verify the proof against the expected chain. All three fields are immutable. |
| Decimals | Import the verified issuance’s decimal display, using the protocol default of zero when the applicable issuance has no decimal display. Reject malformed or unsupported precision. Freeze the scale at admission because stored integers and historical statements depend on it. |
| Code and display ticker | The operator approves a short code and display ticker. Issuer-provided metadata may suggest them. Duplicate codes and tickers are valid. They convey no claim about redemption, reserves or issuer identity. |
| Optional name | Use the verified genesis asset name as the initial label. An operator can approve a display-name correction. Preserve the original issuance evidence. |

A group key can identify multiple asset IDs. Keep the group key as supporting evidence when present. Adding a new tranche creates a separate catalog identity. Combining issuances under one balance would require a future decision on redemption and accounting. The Lightning Labs API distinguishes issuance leaves, individual asset IDs and group keys. [QueryProof](https://lightning.engineering/api-docs/api/taproot-assets/universe/query-proof/)

## Import and reconciliation

1. An operator supplies the expected network and exact asset ID. The provider can discover a candidate from its node or a configured Universe. Discovery does not authorize wallet creation or any credit.
2. The provider verifies the issuance proof on the expected chain, matches the genesis ID, and checks the metadata reveal against the committed metadata hash when a reveal is required. Record the verified genesis reference, metadata hash and provenance. A missing reveal that is needed to establish precision leaves the candidate pending. [Asset metadata](https://docs.lightning.engineering/lightning-network-tools/taproot-assets/asset-metadata)
3. Admit a verified and approved candidate using the existing unique key `(protocol, network, asset_ref)`. Concurrent import and replay return the same catalog UUID. A changed genesis reference, network or precision requires operator review. Leave the existing row and all amounts intact.
4. Treat new display metadata as a proposal. A reviewed label change affects presentation. Proofs arriving from another Universe do not get to edit an approved ticker automatically. An operator removing an asset from the supported set disables new activity while preserving wallet history and statement units.

## Dashboard and operator behavior

Only verified, enabled issuances with an available provider can be selected for a new wallet or transfer. Show protocol, network and an abbreviated asset ID next to ambiguous tickers, with the full copyable ID in details. Display balances and historical activity for an already-held asset during a provider outage. Explain the unavailable action and offer a retry. Operators can inspect the source, verification failure and last successful check. A node outage must never zero or reprice a SwissKnife balance.

The current installation seeds native Bitcoin for its supported networks. Taproot Assets admission stays disabled until a provider implements and passes the checks below. No third-party registry, background catalog service or public write API is needed for this first integration.

## Adapter acceptance

- Mint two regtest issuances with the same label and ticker. Import both, verify distinct wallet UUIDs, and settle the second issuance while the first wallet remains unchanged.
- Replay the same import concurrently on SQLite and PostgreSQL. Preserve the first UUID, scale and wallet funds. Persist a display-only correction and read back the same units.
- Reject an asset on the wrong chain, a mismatched ID, an invalid proof and a mismatched required metadata reveal. Keep unverified candidates out of the selectable catalog.
- Reconcile an unavailable node and a changed metadata proposal. Existing statements remain readable. Recover availability after the same issuance verifies again.
- Exercise the same checks on each supported provider transport through the public API. Record provider and proof fixtures with that adapter.
