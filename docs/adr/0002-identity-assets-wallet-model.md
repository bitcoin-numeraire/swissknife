# ADR 0002: Identity, assets, and asset-scoped wallets

| Field | Value |
| --- | --- |
| Status | Accepted |
| Date | 2026-07-02 |
| Related issues | #297, #252, #254, #291, #292, #115, #237, #239, #240, #241 |
| Scope | Identity, accounts, wallets, currencies/assets, network scoping, migration strategy |

## Summary

SwissKnife should replace the current wallet-as-user model with an explicit identity/account model:

```text
account -> auth_identity
account -> wallet(asset_id)
wallet(asset_id) -> payments, invoices, Bitcoin addresses, Lightning-address receiving route
```

An `account` is the owner and authorization boundary. A `wallet` is a spendable balance for exactly one asset on exactly one settlement network. Payments, invoices, addresses, and balance reservations attach to that wallet, so a money-moving operation cannot reserve one asset/network and settle another. This is a breaking model change: existing data should be migrated forward, but old wallet-as-user API behavior should not be preserved as a product contract.

This ADR supersedes the long-term note in ADR 0001 that deferred the `user + per-currency wallet` split. The split is now the correct target for the Identity & account model epic.

## Current problems

### `wallet` is both user and wallet

Today `wallet.user_id` is the effective user table:

```text
wallet(id, user_id unique, created_at, updated_at)
```

Authentication resolves an external subject to a wallet through `WalletRepository::find_by_user_id`, and the authenticated `User` domain object contains one `wallet_id`. Handlers then default optional `wallet_id` request fields to `user.wallet_id`.

That forces one identity to equal one wallet forever. It also makes `api_key.user_id` reference `wallet.user_id` instead of a real user/account row.

### Balance scope is not the resource scope

ADR 0001 introduced `wallet_balance(wallet_id, currency)` as a tactical accounting fix. That prevents some cross-currency balance mistakes, but the main resources still point at the user-container wallet:

```text
payment.wallet_id -> wallet(id)
payment.currency

invoice.wallet_id -> wallet(id)
invoice.currency
```

The FK does not identify the spendable balance being mutated. Every flow has to carry `wallet_id + currency` separately, which is exactly the ambiguity the remodel should remove.

### `Currency` currently conflates assets and Bitcoin networks

The current `Currency` enum stores values such as `Bitcoin`, `BitcoinTestnet`, `Regtest`, `Simnet`, and `Signet`. Those values mix two concepts:

- the human/economic unit or asset, for example BTC or USDT;
- the settlement network, for example Bitcoin mainnet, testnet, signet, or regtest.

That works for the BTC-only era because testnet/regtest BTC are often treated as different "currencies" (`tBTC`, `rBTC`) in product copy. It breaks once the same semantic asset exists on more than one network. Example: USDT issued as a Taproot Asset on Bitcoin mainnet and USDT issued on a testnet are both "USDT" to a user, but they are not the same spendable asset in the database.

## Terminology

- **Account**: the internal owner/authorization aggregate: the person or organization boundary, never a balance container. It has a stable UUID and owns wallets, API keys, account permissions, and identity-level settings.
- **Auth identity**: a login identity from a provider, for example an OAuth2 subject or a local JWT username. Multiple identities may later point at one account, but the first migration creates one identity per account.
- **Currency**: the human-facing unit/ticker such as BTC or USDT.
- **Network**: the settlement environment such as Bitcoin mainnet, testnet4, signet, or regtest.
- **Asset**: the balance key: a currency on a specific network/protocol, optionally with a protocol-specific asset identifier. Assets, not currencies alone, are what wallets hold. Taproot Assets belongs here as an asset protocol, not as a balance ledger.
- **Wallet**: one spendable balance for one asset. A user with BTC mainnet and USDT Taproot Assets owns two wallets.
- **Settlement rail**: how a transfer of a wallet asset settles, for example Lightning, on-chain Bitcoin, or the internal payment path. The rail is not the balance key.

## Decision

### 1. Add internal account identity

The owner table is named `account`, one word. `user` is a reserved word in Postgres (`SELECT user` is a builtin), so it would need quoting in every raw query and psql session; compounds like `user_account` dodge the keyword but glue together two words that each mean something else in this domain. The naming rule for the whole model is: **account** is the person (owns things, is the authorization boundary), **wallet** is the money (one spendable balance for one asset — the Bitcoin-native word, already used across the API, dashboard, and docs), and **User** survives only as the runtime principal struct produced by authentication, representing the human actor as `account_id + identity + permissions`. Each stored concept gets one single-word name: `account`, `auth_identity`, `wallet`, `asset`.

```text
account(
  id UUID primary key,
  display_name TEXT NULL,
  permissions JSON NOT NULL DEFAULT '[]',
  created_at TIMESTAMP NOT NULL,
  updated_at TIMESTAMP NULL
)

auth_identity(
  id UUID primary key,
  account_id UUID NOT NULL references account(id),
  provider TEXT NOT NULL,       -- jwt, oauth2, future provider names
  subject TEXT NOT NULL,        -- JWT sub / OAuth2 sub / local username
  created_at TIMESTAMP NOT NULL,
  unique(provider, subject)
)

account_preference(
  account_id UUID primary key references account(id),
  dashboard_settings JSON NOT NULL DEFAULT '{}',
  created_at TIMESTAMP NOT NULL,
  updated_at TIMESTAMP NULL
)
```

The domain object returned by authentication should stop meaning "wallet holder" and become account-shaped:

```rust
pub struct User {
    pub account_id: Uuid,
    pub subject: String,
    pub provider: AuthProvider,
    pub permissions: Vec<Permission>,
}
```

Permissions are stored as a JSON array on `account`, matching the existing `api_key.permissions` representation because the current product always loads the full permission set instead of querying individual grants. For OAuth2 identities the IdP's token claims are authoritative and `account.permissions` is not consulted: mirroring claims into the database would create a second source of truth that drifts. For local-credential identities (`provider = jwt`) there is no IdP, so `account.permissions` is authoritative and the bootstrap admin is granted the full permission set. API keys keep their own `permissions` column because a key is an attenuated grant: at creation its permissions must be a subset of the creator's effective permissions, and at authentication the key's stored permissions are used as-is. If a future admin/grant workflow needs row-level querying, audit history, or grant metadata, promote account permissions to a dedicated table at that point.

Preferences that only the dashboard reads live in `account_preference.dashboard_settings` as a versioned JSON document the server stores but never branches on. Promote a preference to a typed column or table only when backend policy, querying, constraints, or cross-device semantics depend on it.

### 2. Model assets separately from network and display currency

Introduce an asset catalog that can represent native BTC and non-BTC assets without overloading `Currency`:

```text
asset(
  id UUID primary key,
  code TEXT NOT NULL,           -- BTC, USDT, ... human display code; issuer metadata, not unique
  name TEXT NULL,
  protocol TEXT NOT NULL,       -- bitcoin, taproot_assets, internal, ...
  network TEXT NOT NULL,        -- bitcoin/mainnet, bitcoin/testnet4, bitcoin/signet, bitcoin/regtest, ...
  asset_ref TEXT NOT NULL,      -- 'native' for chain-native BTC, Taproot Asset ID for TA assets
  display_ticker TEXT NOT NULL, -- BTC, tBTC, USDT, ...
  decimals SMALLINT NOT NULL,   -- stored integer amounts are units of 10^-decimals of one code unit
  metadata JSON NULL,
  created_at TIMESTAMP NOT NULL,
  updated_at TIMESTAMP NULL,
  unique(protocol, network, asset_ref)
)
```

Three deliberate choices in this schema:

- `asset_ref` is `NOT NULL` with `'native'` as the sentinel for chain-native BTC. NULLs are distinct in unique indexes on both Postgres and SQLite, so a nullable `asset_ref` would let two `(bitcoin, bitcoin/mainnet, NULL)` rows coexist and silently break the catalog's identity guarantee. (Postgres 15's `NULLS NOT DISTINCT` is not portable to SQLite; the sentinel is.)
- `unique(protocol, network, asset_ref)` is the only uniqueness constraint, because those three columns fully identify an asset. `code` stays outside every constraint on purpose: it is issuer-supplied display metadata, and for Taproot Assets nothing prevents two distinct issuances from claiming the same ticker.
- `decimals` defines the storage scale, not display rounding: every integer amount for the asset (`wallet.available_amount`, `wallet.reserved_amount`, payment and invoice amounts) is denominated in `10^-decimals` of one `code` unit. Native BTC keeps millisatoshi storage, so `decimals = 11`; a Taproot Asset uses its issuance's decimal-display value. `BIGINT` at `10^-11` BTC still covers ~92M BTC, comfortably above the 21M supply. How many decimals the frontend renders is a display choice bounded by `decimals` and needs no schema field.

The important invariant is that balances key by `asset.id`, not by `code`. `USDT` on mainnet and `USDT` on testnet have the same `code`, but different assets and different wallets.

Balance APIs return wallet balances by `asset_id`. A portfolio or dashboard view may additionally group display values by `(code, network)` — never by `code` alone, which would sum mainnet and testnet units — and even that grouping is presentation-only and must keep protocol/asset-ref breakdowns visible, because two distinct Taproot Asset issuances can claim the same ticker on the same network. Cross-network or cross-asset totals are only meaningful after conversion into a single display currency. Spendability, reservations, debits, credits, and payment validation never aggregate above `asset_id`.

Initial seed assets map the existing enum values into explicit assets:

| Existing value | Target asset |
| --- | --- |
| `Bitcoin` | native BTC on `bitcoin/mainnet` |
| `BitcoinTestnet` | native BTC on the configured Bitcoin testnet network |
| `Signet` | native BTC on `bitcoin/signet` |
| `Regtest` | native BTC on `bitcoin/regtest` |
| `Simnet` | native BTC on `bitcoin/simnet` or the existing regtest-like simnet adapter |

`BitcoinTestnet` currently cannot distinguish testnet3 from testnet4 by itself because `BtcNetwork::Testnet` and `BtcNetwork::Testnet4` both convert to `Currency::BitcoinTestnet`. During migration, use the configured node network when available; if a database has historical mixed testnet3/testnet4 activity, require explicit operator reconciliation rather than silently merging networks.

Network is a database concern, not only a rendering concern. It is persisted exactly once, on `asset`; wallets, payments, invoices, and addresses derive it through their FKs and never duplicate it. It must live in the database for three reasons: payment validation compares the network encoded in the instrument (BOLT11 HRPs `lnbc`/`lntb`/`lntbs`/`lnbcrt`, on-chain address prefixes) against the wallet asset's network; rows must stay attributed to the network they were created on even if the deployment configuration later changes; and the testnet3/testnet4 reconciliation above is only expressible if network is data. What is genuinely frontend-only is pricing and formatting: testnet assets have no market value, so fiat conversion and ticker styling (`tBTC`) are display-layer decisions on top of the persisted network. As an operational guard, startup should verify that the configured node network matches the network of every asset with active wallets and refuse to run on mismatch, instead of discovering the mismatch one rejected payment at a time.

### 3. Make wallets asset-scoped spendable balances

Convert `wallet` from user-container to asset-scoped balance:

```text
wallet(
  id UUID primary key,
  account_id UUID NOT NULL references account(id),
  asset_id UUID NOT NULL references asset(id),
  label TEXT NULL,
  available_amount BIGINT NOT NULL DEFAULT 0 CHECK (available_amount >= 0),
  reserved_amount BIGINT NOT NULL DEFAULT 0 CHECK (reserved_amount >= 0),
  created_at TIMESTAMP NOT NULL,
  updated_at TIMESTAMP NULL,
  unique(account_id, asset_id)
)
```

A user can own many wallets. Each wallet has exactly one asset. This is intentionally stricter than "one wallet row with many `wallet_balance` rows" because payments and invoices need one FK that fully identifies the spendable balance being mutated.

`wallet.asset_id` is immutable after creation: payments and invoices derive their denomination through this FK, so changing it would silently redenominate history. The `CHECK` constraints are a portable backstop on both engines: the conditional reserve `UPDATE` already guards `available_amount`, but a reservation-release bug could otherwise drive `reserved_amount` negative without any error.

`wallet_balance` is dropped as part of the cutover once balances are copied onto asset-scoped wallets; until then it is only a migration source from ADR 0001. If we later need full reconciliation/audit history, add a real ledger-entry or reservation table; do not keep a second authoritative balance table next to `wallet`.

### 4. Payments and invoices derive asset from wallet

`payment.wallet_id` and `invoice.wallet_id` should point at the asset-scoped wallet. The wallet's `asset_id` is the source of truth for what is reserved, debited, or credited.

`payment.currency` and `invoice.currency` are consumed by the migration to map each row onto its asset-scoped wallet, then dropped. The wallet FK is the only source of denomination; API DTOs expose `asset_id` and asset metadata resolved through it. If a request accepts a human currency code for ergonomics, the service must resolve it to one exact asset before any wallet mutation and reject ambiguous input.

The target invariant is:

```text
payment.wallet_id -> wallet(asset_id)
invoice.wallet_id -> wallet(asset_id)
wallet.available_amount/reserved_amount are integers in 10^-decimals of one asset code unit
```

### 5. Addresses and API keys move to account-aware ownership

- `btc_address.wallet_id` remains wallet-scoped. A Bitcoin deposit address belongs to a specific native-BTC wallet on a specific network.
- `ln_address` becomes identity-owned. Add `account_id` while keeping the existing `wallet_id` column as the concrete wallet that receives generated invoices. Address creation should not accept an arbitrary wallet; the service resolves the account's native BTC wallet for the active Bitcoin network and fails if it does not exist. This keeps one username as an identity-level handle while every generated invoice still lands in a concrete wallet compatible with the running node.
- `api_key` should reference `account.id`, not `wallet.user_id`.
- Issue #115's "master key" should be treated as an account/keyring feature, not a side effect of the old wallet-as-user table. A future key model can derive wallet-specific keys from account-level material, but this remodel should not silently create or migrate private key material.

### 6. Account creation must be idempotent

Replace find-then-insert account creation with an idempotent operation:

```text
ensure_account_for_identity(provider, subject, initial_asset_id) -> (account, initial_wallet)
```

The operation should:

1. insert-or-select `auth_identity(provider, subject)`;
2. create the `account` if the identity is new;
3. create the initial wallet for the deployment asset with `ON CONFLICT DO NOTHING` / SQLite equivalent;
4. return the existing rows on conflict instead of surfacing a unique-constraint `500`.

This is the core fix for #291 and supersedes the narrower #254 first-login race. Admin `sign_up`, wallet/account registration, JWT first-login provisioning, and API-key issuance should all share the same conflict policy: no unique-constraint `500`; return success for idempotent same-row creation and `409 Conflict` for genuine semantic duplicates.

### 7. Local JWT deployments should use the same account model

The schema and service layer should support multiple accounts for every auth provider, including local JWT. That avoids baking a single-admin assumption into the remodel.

The first implementation may keep the local JWT product surface small, but it should still create accounts through `auth_identity(provider = jwt, subject = username)` and `account`. Full local multi-user support for Umbrel, desktop, and mobile deployments can be a follow-up product slice because it needs user-management API/UX, password reset/change semantics per user, and permission assignment.

Practical rule:

- **Now:** local JWT identities use the same account and wallet ownership paths as OAuth2/API-key identities.
- **Next:** add local user management on top of `auth_identity(provider = jwt, subject = username)` and a per-account credential store.
- **Never:** special-case the database as if JWT means one user forever.

## API behavior

### Account and wallet endpoints

`/v1/me` should become account-shaped instead of wallet-shaped:

```text
GET /v1/me                -> account profile + auth identities + permissions summary
GET /v1/me/preferences    -> account-scoped dashboard/settings document
PUT /v1/me/preferences    -> replace or patch account-scoped dashboard/settings document
GET /v1/me/wallets        -> wallets owned by the authenticated account
POST /v1/me/wallets       -> create/enable a wallet for an asset the deployment supports
GET /v1/me/wallets/{id}   -> wallet details, ownership checked
```

Money-moving user routes should be wallet-scoped:

```text
GET /v1/me/wallets/{id}/balance
GET /v1/me/wallets/{id}/payments
POST /v1/me/wallets/{id}/payments
GET /v1/me/wallets/{id}/invoices
POST /v1/me/wallets/{id}/invoices
GET /v1/me/wallets/{id}/bitcoin/addresses
POST /v1/me/wallets/{id}/bitcoin/addresses
```

Admin `/v1/wallets` endpoints should stop registering users implicitly. Split them into account management and wallet management:

```text
POST /v1/accounts                 -> create an account / auth identity when admin-managed creation is needed
POST /v1/accounts/{id}/wallets    -> create an asset wallet for that account
GET /v1/wallets                   -> admin list of wallets
```

Exact endpoint naming can be adjusted with dashboard/API review, but the separation must be preserved.

### Explicit wallet selection

For any money-moving request:

1. require an explicit wallet selector, preferably as a path parameter;
2. verify the selected wallet belongs to the authenticated `account` unless the caller has an admin permission that intentionally crosses accounts;
3. infer the expected asset from the wallet and validate the payment/invoice/address input against it;
4. return `400` for a missing wallet selector, `403` for another user's wallet, and `422` for wallet/input asset mismatch.

Examples:

- Paying a mainnet BTC BOLT11 invoice with a testnet BTC wallet returns `422`.
- Paying a USDT Taproot Asset invoice with a BTC wallet returns `422`.
- Passing another user's wallet ID returns `403`, even if the asset matches.

## Asset/network examples

### Native BTC, mainnet

```text
asset.code = BTC
asset.display_ticker = BTC
asset.protocol = bitcoin
asset.network = bitcoin/mainnet
asset.asset_ref = native
```

The wallet may settle over Lightning, on-chain, or the internal payment path. Those are settlement rails; the balance key is still the BTC mainnet asset.

### Native BTC, testnet/signet/regtest

```text
asset.code = BTC
asset.display_ticker = tBTC or rBTC
asset.protocol = bitcoin
asset.network = bitcoin/testnet4 | bitcoin/signet | bitcoin/regtest
asset.asset_ref = native
```

These assets must never be summed for spendability. A portfolio UI may display them near each other, but reserve/debit/credit queries key by `asset_id`.

### Taproot Assets USDT on mainnet and testnet

```text
asset.code = USDT
asset.protocol = taproot_assets
asset.network = bitcoin/mainnet
asset.asset_ref = <mainnet Taproot Asset ID>
```

```text
asset.code = USDT
asset.protocol = taproot_assets
asset.network = bitcoin/testnet4
asset.asset_ref = <testnet Taproot Asset ID>
```

Both display as USDT, but they are different assets and therefore different wallets. The provider adapter maps `asset_ref + network` into the Taproot Assets node/client calls; application services only need the selected `asset_id` and wallet ownership. If Taproot Asset transfers later support multiple settlement paths, model those as rails for the selected asset, not as separate balance keys.

## Migration strategy

The migration must support SQLite and Postgres and must preserve existing wallets, invoices, payments, balances, addresses, API keys, and Lightning addresses.

### Phase 1: Add identity and asset tables

1. Create `account`, `auth_identity`, `account_preference`, and `asset`.
2. Seed assets for all existing `Currency` enum values.
3. Backfill one `account` per distinct `wallet.user_id`.
4. Backfill one `auth_identity` per old wallet user ID using the configured auth provider when known; otherwise require an operator-supplied provider mapping before migration.
5. Add `account_id` to `api_key` and backfill through `wallet.user_id`.
6. Grant the full permission set in `account.permissions` to the bootstrap admin's account when the deployment uses local JWT. OAuth2 deployments backfill nothing, because token claims stay authoritative. `api_key.permissions` rows are unchanged.

### Phase 2: Convert wallet rows into asset wallets

1. Add `wallet.account_id`, `wallet.asset_id`, `wallet.available_amount`, `wallet.reserved_amount`, and `wallet.label`.
2. Build a temporary mapping table:

   ```text
   wallet_asset_migration(old_wallet_id, old_currency, new_wallet_id)
   ```

3. For each `(old_wallet_id, currency)` present in `wallet_balance`, `payment`, or `invoice`, create exactly one asset-scoped wallet.
4. Backfill wallet balances from `wallet_balance.available_amount` and `wallet_balance.reserved_amount`. If no `wallet_balance` row exists, derive the same values from settled invoices, settled payments, and pending reservations using the ADR 0001 formula.
5. Fail loudly if balances would become negative in a way the new model cannot represent.

### Phase 3: Repoint resource FKs

1. Update `payment.wallet_id` by joining `(old wallet_id, payment.currency)` through the migration mapping.
2. Update `invoice.wallet_id` by joining `(old wallet_id, invoice.currency)` through the migration mapping.
3. Move `btc_address.wallet_id` to the account's native BTC wallet for the active Bitcoin network. Existing `btc_address` rows do not carry their own currency, so this must use deployment/network configuration or a documented manual fallback.
4. Add `ln_address.account_id` and keep the old `wallet_id` column as the invoice-receiving wallet route.
5. Resolve the receiving wallet from `(account_id, active native BTC asset)` in service code and cover it with integration tests. This avoids accepting mismatched wallets or networks at the API boundary.

### Phase 4: Cut over services

1. Replace `WalletRepository::find_by_user_id` with account/identity repository operations.
2. Add wallet repository methods such as:

   ```rust
   async fn find_by_account_and_asset(account_id: Uuid, asset_id: Uuid) -> Result<Option<Wallet>, DatabaseError>;
   async fn upsert(account_id: Uuid, asset_id: Uuid) -> Result<Wallet, DatabaseError>;
   ```

3. Move reserve/debit/credit operations from `wallet_balance` to `wallet` conditional updates:

   ```sql
   UPDATE wallet
   SET available_amount = available_amount - :reserve_amount,
       reserved_amount = reserved_amount + :reserve_amount
   WHERE id = :wallet_id
     AND asset_id = :asset_id
     AND available_amount >= :reserve_amount;
   ```

4. Update `AuthService` to call `ensure_account_for_identity` and return account-shaped `User` data.
5. Update payment and invoice services to resolve a wallet first, then derive asset/currency from that wallet.
6. Update internal payment self-payment checks to compare account and wallet intentionally:
   - same wallet: always reject;
   - same account, different asset wallet: reject unless a future conversion/swap feature explicitly supports it;
   - different accounts, same asset: allowed for internal payments.

### Phase 5: API, dashboard, and cleanup

1. Update shared DTOs in `crates/swissknife-types` to expose wallet asset metadata and remove currency-as-source-of-truth fields.
2. Regenerate OpenAPI and dashboard client types.
3. Update dashboard wallet selection to show account wallets by asset/network.
4. Drop `wallet_balance`, `wallet.user_id`, and the payment/invoice currency columns as part of the cutover.
5. Remove old wallet registration semantics.

## Testing strategy

### Migration/backfill tests

Cover existing databases containing:

- one user with no activity;
- one user with settled invoices and settled payments;
- pending outgoing payments with `reserved_amount`;
- multiple old `wallet_balance` currencies under one old wallet;
- API keys referencing old `wallet.user_id`;
- Lightning addresses and generated invoices;
- Bitcoin addresses;
- an imported `BitcoinTestnet` database where network choice must come from configuration.

### Authorization tests

- A user cannot read, pay from, invoice from, or create addresses for another user's wallet by passing an explicit `wallet_id`.
- Admin routes can intentionally cross accounts only with the existing write/read permissions.
- API keys resolve to `account.id` and inherit permissions without relying on `wallet.user_id`.

### Asset/network invariant tests (#292)

- Mainnet BTC, testnet BTC, signet BTC, and regtest BTC balances are never summed for spendability.
- Two wallets with the same display currency code on different networks cannot satisfy each other's payments.
- USDT Taproot Asset wallets on different networks are distinct even when `code = USDT`.
- A BOLT11 invoice or on-chain address whose network does not match the selected wallet returns `422`.

### Idempotency/concurrency tests (#291)

- Concurrent JWT first-login/account provisioning returns one account and one initial wallet, no `500`.
- Concurrent local sign-up/admin creation returns one success and deterministic conflict/idempotent responses, no unique-constraint `500`.
- Concurrent API-key creation and wallet creation handle conflicts as documented.

### Accounting tests

- Reservations and debits mutate `wallet.available_amount/reserved_amount` atomically.
- Failed payments release reservation on the same asset wallet.
- Settled payments adjust reservation to actual fee on the same asset wallet.
- Incoming invoice settlement credits exactly once.
- Internal payments debit sender and credit receiver in one Unit of Work.

## Implementation sequence

1. Land this ADR for #297.
2. Add the identity, permission, preference, and asset tables plus backfill scaffolding.
3. Introduce account/identity repositories and idempotent account provisioning for local JWT and OAuth2.
4. Convert wallets to asset-scoped rows and migrate balances/resources.
5. Cut over payment, invoice, wallet, address, API-key, and auth services to account + asset wallet lookups.
6. Replace wallet-shaped `/v1/me` behavior with account-shaped `/v1/me`, wallet-scoped user routes, and admin account/wallet management endpoints.
7. Regenerate OpenAPI/dashboard types and update dashboard wallet selection.
8. Add the migration, authorization, asset/network, idempotency, and accounting tests.
9. Drop `wallet_balance`, `wallet.user_id`, and currency-as-source-of-truth code as part of the cutover.

Each step should be a focused PR. The wallet conversion and service cutover may need to land together if the compiler cannot support a meaningful intermediate state, but the migration should still be staged and tested as above.

## Consequences

### Benefits

- Identity becomes independent from spendable wallets.
- A user can own multiple wallets without overloading a single `wallet.user_id` row.
- The balance key is explicit and network-safe.
- Taproot Assets and examples like USDT on mainnet/testnet fit the same model as native BTC.
- Payment and invoice FKs identify the exact wallet to reserve, debit, or credit.
- JWT and OAuth2 deployments use the same internal account model.
- The account-creation race is fixed at the aggregate boundary instead of patched one endpoint at a time.

### Costs

- Requires a careful multi-table migration and likely temporary migration columns.
- Requires a breaking OpenAPI/dashboard change because wallet responses need asset metadata and account endpoints change shape.
- Requires clear product decisions for local JWT multi-user UX before exposing it fully.

## Open follow-ups

- Design the local JWT multi-user API/UX for Umbrel, desktop, and mobile deployments.
- Decide the exact asset catalog source of truth for Taproot Assets metadata once a Taproot Assets adapter is implemented.
- Design the account/master-key model for #115 without assuming one old wallet equals one user.
