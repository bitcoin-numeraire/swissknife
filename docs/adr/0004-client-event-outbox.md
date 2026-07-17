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

Expose `GET /v1/me/wallets/{wallet_id}/events` as `text/event-stream`.

SSE fits the one-way workload, works through ordinary HTTP infrastructure, and has a standard event cursor. The generated fetch client is used instead of the browser `EventSource` API so JWT and API-key `Authorization` headers remain available. The endpoint is excluded from the normal request timeout, emits a heartbeat every 15 seconds, disables nginx response buffering, and uses the existing permissive CORS policy.

The authenticated principal must have `read:transaction`, and the selected wallet must belong to that principal's account. There is no account-wide unfiltered stream.

### Commit a durable event in the state-change transaction

Add a `client_event` table containing a monotonic ID, wallet scope, stable event type, resource ID, JSON snapshot, and creation time. The payment and event-projection units of work append the event before committing the same transaction that changes the payment or invoice and its wallet balance.

This is a transactional outbox: after a successful commit, both state and event exist; after a rollback, neither exists. A unique `(event_type, resource_id)` index makes listener replays idempotent while still permitting a failed payment to be corrected later by a distinct `payment.settled` event.

The initial event vocabulary is:

- `invoice.paid`
- `payment.settled`
- `payment.failed`

The payload is the full public invoice or payment snapshot committed by that transition. Internal fields such as balance reservations and encrypted LNURL success actions remain excluded by their existing serialization rules.

### Provide at-least-once replay

A fresh connection starts after the wallet's latest event, so it observes new changes without replaying its entire history. A reconnect sends `Last-Event-ID`; a client that deliberately wants history may send `after`. Events are returned in increasing ID order.

The server checks the shared database once per second when a stream is idle. This adds at most one second of delivery latency, works for SQLite and PostgreSQL, and catches events committed by any application replica without requiring replica-local pub/sub. Heartbeats address proxy idle timeouts but are not durable events and carry no ID.

Delivery is at least once: a disconnect after a client receives an event but before it persists the cursor can cause replay. Consumers must use the event ID for deduplication. The dashboard safely responds by revalidating idempotent SWR cache keys.

### Reuse the log for signed webhooks

Webhook delivery consumes this same durable event log rather than creating a second set of settlement hooks. Account-scoped CRUD endpoints under `/v1/me/wallets/{wallet_id}/webhooks` manage an HTTPS endpoint and a non-empty event filter. A new subscription starts at the current event cursor; it does not unexpectedly replay historical payments. Disabling a subscription exhausts pending attempts and advances its cursor, so re-enabling it resumes with new events rather than producing a backlog. An attempt already claimed before the disable may still complete.

Subscriptions and delivery-attempt state live in separate tables. A background worker first fans matching outbox rows into unique `(subscription_id, client_event_id)` delivery rows, then claims due work with a 60-second database lease. This supports multiple application replicas and keeps all external I/O outside wallet settlement transactions. Delivery is at least once: an endpoint must deduplicate using `X-SwissKnife-Delivery`, and event order is not guaranteed across concurrent attempts.

The JSON body contains the stable event ID and type, wallet and resource IDs, timestamp, and committed public snapshot. Requests include:

- `X-SwissKnife-Event`
- `X-SwissKnife-Delivery`
- `X-SwissKnife-Timestamp`
- `X-SwissKnife-Signature: v1=<hex HMAC-SHA256>`

The signed message is `<timestamp>.<raw request body>`. A random 256-bit base64url secret is returned only on subscription creation or explicit rotation. Consumers should reject old timestamps and compare signatures in constant time. A rotation affects subsequent attempts; an attempt already claimed by a worker may still carry the previous signature.

Only public HTTPS destinations are delivered. The worker rejects credentials and fragments, resolves DNS itself, rejects any private, loopback, link-local, multicast, or reserved result, pins the verified address for the request, disables redirects, and uses a ten-second timeout. Network failures, HTTP 408/409/425/429, and 5xx responses retry exponentially from one minute up to one hour. Other non-2xx responses are permanent failures. Delivery exhausts after eight attempts and remains visible through the delivery-history endpoint.

## Consequences

- Clients no longer poll wallet resources to notice settlement.
- Listener replay, synchronous/listener races, process restarts, and multiple application replicas preserve one committed event per transition.
- Event history begins when this migration is deployed; existing terminal payments and invoices are not backfilled.
- The event log currently has no retention job. Retention can be added only with an explicit minimum replay window and webhook-delivery watermark so undelivered events are never removed.
- Webhook signing secrets are stored in the application database because SwissKnife has no deployment-wide envelope-encryption facility today. Database access must therefore be treated as secret access; a future key-management integration can encrypt the column without changing the wire contract.
- WebSocket support is deferred. It should be added only if a real bidirectional protocol appears; deployment in separate pods alone is not a reason to maintain two transports.
