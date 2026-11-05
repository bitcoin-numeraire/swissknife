# ADR 0004: Durable client event log and server-sent events

| Field | Value |
| --- | --- |
| Status | Accepted |
| Date | 2026-07-17 |
| Related issues | #302, #256, #130, #293 |
| Scope | Settlement event durability, client push, replay, authentication, webhook fan-out |

## Context

SwissKnife already receives Lightning and on-chain events, but external clients have to poll REST resources to discover asynchronous settlement. A process can also receive the same node event more than once, and the synchronous payment RPC can race the node listener. Client push therefore needs the same idempotency and crash consistency as wallet accounting.

The stream is server-to-client only. Clients never send commands over it; commands remain ordinary authenticated HTTP requests.

## Decision

### Use SSE for the client stream

Expose `GET /v1/me/events` as `text/event-stream`.

SSE fits the one-way workload, works through ordinary HTTP infrastructure, and has a standard event cursor. The generated fetch client is used instead of the browser `EventSource` API so JWT and API-key `Authorization` headers remain available. The normal request timeout bounds authentication and initial cursor lookup; it ends once response headers are ready and does not limit the SSE body. The endpoint emits a heartbeat every 15 seconds, disables nginx response buffering, and uses the existing permissive CORS policy.

Like the other `/me` endpoints, the stream requires authentication and no administrative permissions. It is scoped by the authenticated account and includes events from all wallets owned by that account. Open streams reauthenticate their credentials every 15 seconds, closing when tokens expire or access is revoked. Every event retains its `wallet_id`, so one connection can drive an account-wide dashboard or client without leaking events across accounts.

### Commit a durable event in the state-change transaction

Add a `client_event` table containing a monotonic ID, wallet scope, stable event type, resource ID, JSON snapshot, and creation time. The payment and event-projection units of work append the event before committing the same transaction that changes the payment or invoice and its wallet balance.

Event appends acquire a shared database write lock until their transaction commits, so concurrent PostgreSQL transactions cannot expose a higher cursor before a lower cursor. Retention takes the same lock.

This is a transactional outbox: after a successful commit, both state and event exist; after a rollback, neither exists. A unique `(event_type, resource_id)` index makes listener replays idempotent while still permitting a failed payment to be corrected later by a distinct `payment.settled` event.

The initial event vocabulary is:

- `invoice.pending`, emitted once when the selected node provider reports an on-chain deposit in the mempool
- `invoice.paid`
- `payment.settled`
- `payment.failed`

The payload is the full public invoice or payment snapshot committed by that transition. An on-chain deposit keeps the same invoice resource ID as it moves from `invoice.pending` to `invoice.paid`; the pending snapshot includes its unconfirmed Bitcoin output. Internal fields such as balance reservations and encrypted LNURL success actions remain excluded by their existing serialization rules.

LND reports zero-confirmation wallet transactions, so its on-chain lifecycle includes both events. Core Lightning currently registers wallet deposits only after confirmation; without a separate chain data source, its first observable transition remains `invoice.paid`.

### Provide at-least-once replay

A fresh connection starts after the account's latest retained event, so it observes new changes without replaying its entire history. A reconnect sends `Last-Event-ID`; a client that deliberately wants history may send `after`. Events are returned in increasing ID order.

The client retains the last durable event ID outside an individual stream instance. It reopens the stream with that cursor after both transport errors and a clean response-body end, covering proxy shutdowns and rolling server restarts that surface as an ordinary EOF rather than a fetch error. Each successful connection also revalidates REST state after the server establishes its cursor, closing the snapshot-to-stream handoff window.

The server checks the shared database once per second when a stream is idle. This adds at most one second of delivery latency, works for SQLite and PostgreSQL, and catches events committed by any application replica without requiring replica-local pub/sub. Heartbeats address proxy idle timeouts but are not durable events and carry no ID.

Delivery is at least once: a disconnect after a client receives an event but before it persists the cursor can cause replay. Consumers must use the event ID for deduplication. The dashboard safely responds by revalidating idempotent SWR cache keys.

### Bound replay storage and make cursor expiry explicit

Retain events for a configurable minimum window (`client_events.retention`, 30 days by default) and prune them on a configurable interval (`client_events.cleanup_interval`, one hour by default). A zero retention disables pruning; a zero cleanup interval disables the worker. Cleanup is serialized across application replicas and records a monotonic durable watermark in the shared database.

When `Last-Event-ID` or `after` is before that watermark, the server returns `409 Conflict` before opening the stream. The client must then refresh authoritative REST state and reconnect without a cursor. A fresh stream starts no earlier than the watermark, including for a new account with no retained events. The server checks the watermark both before and after each replay query so pruning cannot silently create a partial replay batch.

### Reuse the log for signed webhooks

Webhook delivery consumes this same durable event log rather than creating a second set of settlement hooks. Account-scoped CRUD endpoints under `/v1/me/wallets/{wallet_id}/webhooks` manage an HTTPS endpoint and a non-empty event filter. A new subscription starts at the current event cursor; it does not unexpectedly replay historical payments. Disabling a subscription exhausts pending attempts and advances its cursor, so re-enabling it resumes with new events rather than producing a backlog. An attempt already claimed before the disable may still complete.

Subscriptions and delivery-attempt state live in separate tables. A background worker first fans matching outbox rows into unique `(subscription_id, client_event_id)` delivery rows, then claims due work with a 60-second database lease. Attempt outcomes are applied only while that exact lease is still current, so a late worker cannot overwrite the result of a newer claim. This supports multiple application replicas and keeps all external I/O outside wallet settlement transactions. Delivery is at least once: an endpoint must deduplicate using `X-SwissKnife-Delivery`, and event order is not guaranteed across concurrent attempts.

The JSON body contains the stable event ID and type, wallet and resource IDs, timestamp, and committed public snapshot. Requests include:

- `X-SwissKnife-Event`
- `X-SwissKnife-Delivery`
- `X-SwissKnife-Timestamp`
- `X-SwissKnife-Signature: v1=<hex HMAC-SHA256>`

The signed message is `<timestamp>.<raw request body>`. A random 256-bit base64url secret is returned only on subscription creation or explicit rotation. Consumers should reject old timestamps and compare signatures in constant time. A rotation affects subsequent attempts; an attempt already claimed by a worker may still carry the previous signature.

The `/me/wallets/{wallet_id}/webhooks` endpoints authenticate the account and verify wallet ownership, like the other account wallet operations. Ordinary accounts can manage their subscriptions without administrative permissions. `GET /me/webhooks` lists subscriptions across the authenticated account's wallets. Account-scoped list filters cannot override the authenticated account or the wallet in the path, even when the caller also holds administrative permissions.

Administrative endpoints under `/v1/webhooks` follow the API-key handler's permission model. `read:webhook` permits listing subscriptions across accounts, fetching one subscription, and viewing its delivery history. `write:webhook` permits creation, updates, enable/disable, deletion, and secret rotation. Read and write permissions are independent. Creation requires an explicit `wallet_id` and derives the subscription's owner from that wallet; subsequent edits cannot transfer ownership. Both route families use the same services, validation, and delivery worker. List filters support account, wallet, IDs, enabled state, ordering, limit, and offset.

New local bootstrap administrators receive both webhook permissions through `Permission::all_permissions()`. Existing local administrators can be granted the new permissions through account-permission management; OAuth2 deployments must grant the corresponding scopes in their identity provider. Existing API keys retain their explicitly granted scopes.

Only public HTTPS destinations are delivered. The worker rejects credentials and fragments, resolves DNS itself, rejects any private, loopback, link-local, multicast, or reserved result, pins the verified address for the request, disables redirects and environment proxies, and bounds DNS resolution and the complete request by ten seconds. Network failures, HTTP 408/409/425/429, and 5xx responses retry exponentially from one minute up to one hour. Other non-2xx responses are permanent failures. Delivery exhausts after eight attempts and remains visible through the delivery-history endpoint.


Retention takes the same event-log lock as fan-out and subscription changes. It preserves events not yet consumed by an active subscription and events referenced by pending deliveries. Terminal delivery records are removed with their expired events; otherwise their foreign keys would prevent cleanup. Subscription updates modify only requested fields and cannot roll back a worker cursor or a rotated secret.

### Dashboard diagnostics and explicit delivery actions

The Developers dashboard separates personal and instance scope for both API keys
and webhooks. Personal operations use `/me` even for administrators. Administrative
read and write permissions remain independent; write-only users supply known IDs
without fetching other accounts or wallets. Secrets remain in transient component
state until acknowledged, and are never stored in list/detail caches.

Both webhook route families expose filtered, paginated `GET /{id}/deliveries`,
`GET /{id}/deliveries/{delivery_id}` with the retained signed payload,
`POST /{id}/test`, and `POST /{id}/deliveries/{delivery_id}/retry`. Reads require
ownership or `read:webhook`; actions require ownership or `write:webhook`.

Test deliveries use the ordinary durable delivery queue, destination checks,
signing, leases, timeout and retry policy. Their payload type is `webhook.test`,
the event ID is a UUID, and `resource_id` is absent. They never enter the wallet
event journal or SSE stream, change balances, or fan out to other subscriptions.
Each subscription allows one test per minute and must finish its previous test
before queuing another. Disabling a subscription also stops queued tests.

Manual redelivery only queues retained terminal deliveries on enabled
subscriptions, with no live lease and fewer than eight recorded attempts. It
preserves the delivery ID, original payload and cumulative attempt count; it does
not reset the budget or replay an event to other subscriptions. A successful
redelivery uses the current destination and signing secret. Receivers must still
deduplicate the stable delivery ID. Disabling retains outstanding leases so a
subsequent re-enable/retry cannot overlap a request already in flight. Concurrent
manual actions serialize with retention and subscription changes.

Test payloads are stored directly on delivery rows, with a database constraint
requiring exactly one source: a wallet event or a test payload. Terminal tests
expire under the same retention window as wallet events; pending tests survive
cleanup. Upgrading preserves existing delivery IDs, attempt history and foreign
key behavior on both SQLite and PostgreSQL.

## Consequences

- Clients can stop polling wallet resources solely to notice settlement. REST remains authoritative for initial loads, explicit refresh, focus, and navigation.
- Listener replay, synchronous/listener races, process restarts, and multiple application replicas preserve one committed event per transition.
- Event history begins when this migration is deployed; existing terminal payments and invoices are not backfilled.
- Event replay is bounded. A client offline longer than the configured window receives an explicit reset signal and must rebuild state from REST.
- Webhook signing secrets are stored in the application database because SwissKnife has no deployment-wide envelope-encryption facility today. Database access must therefore be treated as secret access; a future key-management integration can encrypt the column without changing the wire contract.

- WebSocket support is deferred. It should be added only if a real bidirectional protocol appears; deployment in separate pods alone is not a reason to maintain two transports.
