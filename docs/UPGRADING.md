# Upgrading from 0.2.0 to 0.3.0

Version 0.3.0 changes the account/wallet API and database schema. Upgrade the
backend, dashboard, and API integrations together.

## Database migration

The rehearsed production upgrade is PostgreSQL with OAuth2 identities and
native BTC mainnet wallets. Existing wallet IDs, available and reserved balances,
transaction history, addresses, and API-key credentials are preserved. Legacy
API keys are assigned to their existing owners' new accounts; no key deletion or
reissuance is required.

Fresh installations support both PostgreSQL and SQLite. Existing SQLite,
local-JWT, or non-mainnet legacy databases are outside this PostgreSQL backfill
and need a separate migration assessment before upgrading.

For an unchanged 0.2.0 PostgreSQL database, the migration table has 16 entries,
ending at `m20260609_143601_backfill_wallet_balances`. The 0.3.0 schema has 31
entries, ending at `m20260814_151430_promote_wallet_account_unique_constraint`.
Check the actual database rather than relying only on its image tag:

```sql
SELECT version, applied_at FROM seaql_migrations ORDER BY version;
```

The new backend applies pending migrations automatically before accepting HTTP
requests. The account cutover removes legacy columns and is irreversible.
Stop all old backend instances and take a fresh backup before starting 0.3.0;
do not run 0.2.0 and 0.3.0 against the same database during a rolling upgrade.

A rollback requires restoring the pre-upgrade database and the old images.
Changing only the Helm image tag cannot undo the schema change. If new writes
have occurred, reconcile them before restoring a backup.

## API integrations

- `GET /v1/me` now returns an account with its wallet summaries.
- Select a wallet explicitly. Balances, payments, invoices, contacts, and Bitcoin
  addresses use `/v1/me/wallets/{wallet_id}/...` routes.
- Wallets carry `account_id` and `asset_id`; amounts and network validation use
  the wallet's asset. Removed legacy currency and implicit-wallet contracts are
  not compatibility aliases.
- Administrative account management uses `/v1/accounts`. Creating an account
  there does not create a login identity.
- Administrative bulk DELETE requests require an effective row-selection
  filter. Pagination alone does not select rows and returns HTTP 400.
- Fee quotes are estimates. Lightning sends enforce a maximum fee; on-chain
  sends prepare again and can have a different final fee.

Use `/docs` and the regenerated OpenAPI client for the complete contract.

## Authentication and permissions

For OAuth2/Auth0 deployments, define `read:account` and `write:account` on the API
and grant them to administrator roles before deployment. Token claims remain the
authority for OAuth2 permissions; changing database account permissions does not
replace IdP grants.

Keep existing permissions, including `read:btc_address` and `write:btc_address`,
which protect administrative Bitcoin-address routes. Account-scoped wallet
routes check ownership instead of requiring global administrative permissions.
Sign in again and verify the permissions in a fresh access token.

Local JWT still provides the installation's owner login. Per-account local
credential management is not part of this release.

## Configuration changes

- Remove `fee_buffer`, `maxfeepercent`, and `payment_exemptfee` from deployment
  overrides. Use the selected provider's absolute Lightning fee policy:
  `maxfee` for CLN or `fee_limit_msat` for LND, in millisatoshis. Unset CLN
  `maxfee` uses `xpay`'s `max(1%, 5000 msat)` policy.
- The provider baseline used for this release is CLN `v26.06.9` and LND
  `v0.21.4-beta`. CLN payments and quotes require `xpay` and `getroutes` access;
  confirm that the configured rune or gRPC credentials permit the needed RPCs.
- Client-event retention defaults to 30 days, with hourly pruning. The stream
  starts at deployment; old terminal transactions are not backfilled as events.
  Ensure the ingress supports long-lived SSE responses without buffering.
- Dashboard public settings are build-time values. Match its API URL, Auth0
  audience, and authentication method to the backend when building the image.

## Deployment checks

Publish and verify the matching immutable images before updating k8s-infra.
Update backend and dashboard image tags and chart `version` / `appVersion`
together, then deploy with old backend replicas stopped for the schema cutover.

Verify migration completion, `/v1/system/health`, `/v1/system/ready`, and
`/v1/system/version`. Sign in and check wallet balances, history, Accounts and
Bitcoin Addresses administration, and live settlement updates. Before accepting
traffic, compare ownership and financial totals with the pre-upgrade snapshot.

An ambiguous provider send or broadcast remains pending with its funds reserved.
Startup/reconnect reconciliation can recover a final node result, but absence
of a final result can require manual reconciliation. Do not automatically resend
or release a reservation merely because an HTTP request failed.
