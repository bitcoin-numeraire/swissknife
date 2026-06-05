# ADR 0001: AppStore, Unit of Work, and wallet reservations

| Field | Value |
| --- | --- |
| Status | Proposed |
| Date | 2026-06-05 |
| Related issues | #15, #229, #235, #236, #237, #238, #239, #240, #241 |
| Scope | Backend service dependency injection, repository ports, database transactions, wallet accounting |

## Summary

SwissKnife should split dependency injection from database transaction mechanics and introduce a durable wallet balance/reservation model before the next production-facing release.

The target architecture is:

1. `AppStore` becomes a pure trait container. It contains repository ports, focused Unit-of-Work ports, and a database health probe. It does not own `DatabaseConnection`, expose `begin()`, or construct SeaORM repositories.
2. SeaORM store construction moves to the infrastructure layer.
3. Domain/application repository traits stop exposing `sea_orm::DatabaseTransaction`.
4. Payment-critical multi-write flows use named `PaymentUnitOfWork` methods rather than service-visible transactions.
5. Wallet balance safety moves from aggregate `SUM()` checks to a materialized balance/reservation model with atomic conditional updates that work on Postgres and SQLite.
6. Event projection receives a separate focused Unit-of-Work after the payment-critical path is corrected.

This keeps the business layer responsible for policy decisions while infrastructure owns transaction mechanics and database-specific execution.

## Current problems

### `AppStore` mixes unrelated responsibilities

Today `AppStore` acts as:

- a repository trait container;
- a SeaORM repository factory;
- the owner of a concrete `DatabaseConnection`;
- a transaction manager through `begin()`;
- a health probe through `ping()`.

That blocks service-level unit tests: generated `mockall` repository mocks exist, but services require a concrete `AppStore` whose only constructor builds real SeaORM repositories from a real DB connection.

It also violates the intended dependency direction. `application/entities/store.rs` imports concrete SeaORM infrastructure repositories, so application wiring points outward into infra.

### Transactions leak into repository ports

`PaymentRepository::insert` and `WalletRepository::get_balance` currently accept `Option<&DatabaseTransaction>`. That imports SeaORM into domain/application-facing repository traits.

Repository ports should describe data capabilities, not concrete transaction handles. Transaction mechanics should be hidden behind application-level Unit-of-Work ports implemented by infrastructure.

### Current transaction boundaries are ineffective in key flows

`PaymentService::send_internal` opens an outer transaction, inserts the receiver invoice through a repository method that does not receive that transaction, then calls `insert_payment`, which opens and commits its own separate transaction. The outer transaction is therefore decorative.

The result is not atomic. A settled receiver invoice can be inserted even if the payer-side payment later fails due to insufficient funds.

Internal Bitcoin payments and internal Bolt11 payments have the same shape: payment and counterpart invoice writes are split across independent repository calls.

### Aggregate balance checks do not provide a strong invariant

Current wallet balance is derived from aggregate queries over settled invoices and pending/settled payments. `insert_payment` reads the aggregate balance, compares it with a required amount, and inserts a payment.

Under Postgres `READ COMMITTED`, two concurrent transactions can both read the same available balance and both insert outgoing payments. SQLite has lower expected contention and serializes writers, but the model should still be deterministic and tested there.

For company/server deployments, Postgres is the urgent correctness target. A balance invariant should not rely on aggregate reads plus implicit isolation behavior.

## Verified note on `fee_buffer`

The current `fee_buffer` behavior was added in PR #78 as `fee_buffer = 0.02`. The code and history show it is an admission-control buffer for outgoing payments whose final fee is not known when the pending payment row is created.

Current behavior:

```rust
let required_balance_msat = if let Some(fee_msat) = payment.fee_msat {
    payment.amount_msat + fee_msat
} else {
    payment.amount_msat * (1.0 + fee_buffer)
};
```

Important implications:

- It does **not** increase the payment amount.
- It does **not** record a service fee or markup.
- It does **not** persist a reservation for the buffered amount.
- Once a pending Lightning payment is inserted with `fee_msat = None`, current balance calculations subtract only the payment amount and `COALESCE(fee_msat, 0)`.
- On-chain payments with known prepared fees now pass an explicit `fee_msat` and use `fee_buffer = 0.0`.
- Internal payments pass `fee_msat = Some(0)` and use `fee_buffer = 0.0`.

So the current buffer is best understood as a temporary headroom check for unknown Lightning routing fees, not as a product-level service fee charged to the user.

This ADR intentionally does **not** preserve `fee_buffer` as an ambiguous hidden accounting rule. The follow-up implementation should either:

1. remove it from payment accounting if it is no longer needed; or
2. replace it with a documented fee-reservation policy, for example `estimated_fee_reserve_msat`, `fee_reserve_ratio`, or `max_fee_reserve_msat`, whose reserved amount is persisted and released/adjusted when the payment settles or fails.

If SwissKnife later needs a service fee, spread, or anti-gaming charge, that should be modeled as a separate explicit product/accounting concept, not overloaded onto `fee_buffer`.

## Decision

### 1. Make `AppStore` a pure dependency container

Target shape:

```rust
#[derive(Clone)]
pub struct AppStore {
    pub ln_address: Arc<dyn LnAddressRepository>,
    pub payment: Arc<dyn PaymentRepository>,
    pub invoice: Arc<dyn InvoiceRepository>,
    pub wallet: Arc<dyn WalletRepository>,
    pub api_key: Arc<dyn ApiKeyRepository>,
    pub config: Arc<dyn ConfigRepository>,
    pub btc_address: Arc<dyn BtcAddressRepository>,
    pub btc_output: Arc<dyn BtcOutputRepository>,
    pub payment_uow: Arc<dyn PaymentUnitOfWork>,
    pub event_uow: Arc<dyn EventProjectionUnitOfWork>,
    pub health: Arc<dyn HealthProbe>,
}
```

`AppStore` should not contain:

- `DatabaseConnection`;
- `DatabaseTransaction`;
- `begin()`;
- `ping()`;
- concrete SeaORM repository construction.

Production construction moves to infrastructure, for example:

```rust
// src/infra/database/sea_orm/store.rs
pub fn sea_orm_store(db: DatabaseConnection) -> AppStore {
    AppStore {
        payment: Arc::new(SeaOrmPaymentRepository::new(db.clone())),
        wallet: Arc::new(SeaOrmWalletRepository::new(db.clone())),
        // ...
        payment_uow: Arc::new(SeaOrmPaymentUnitOfWork::new(db.clone())),
        event_uow: Arc::new(SeaOrmEventProjectionUnitOfWork::new(db.clone())),
        health: Arc::new(SeaOrmHealthProbe::new(db)),
    }
}
```

This is an intermediate step. Long-term, services can move from the full `AppStore` to service-specific dependency structs, but a pure `AppStore` is enough to unblock mock-based service tests without a large constructor churn.

### 2. Split database health from the store

Replace `AppStore::ping()` with a health port:

```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn ping(&self) -> Result<(), DatabaseError>;
}
```

`SystemService::health_check` depends on `store.health.ping()` or a service-specific dependency containing the health probe.

### 3. Remove SeaORM transactions from repository traits

Repository trait signatures should be transaction-free:

```rust
#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn insert(&self, payment: Payment) -> Result<Payment, DatabaseError>;
}

#[async_trait]
pub trait WalletRepository: Send + Sync {
    async fn get_balance(&self, wallet_id: Uuid) -> Result<Balance, DatabaseError>;
}
```

SeaORM implementations may share helpers generic over `sea_orm::ConnectionTrait` so normal repositories can call helpers with `&DatabaseConnection` and Unit-of-Work implementations can call the same helpers with `&DatabaseTransaction`. That generic helper detail stays in infra.

### 4. Use focused Unit-of-Work ports with named atomic operations

Do not expose `begin()` / `commit()` to services. A service can decide which operation must be atomic, but not how the DB transaction is opened or committed.

Payment flows should use a focused port:

```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PaymentUnitOfWork: Send + Sync {
    async fn reserve_outgoing_payment(
        &self,
        payment: Payment,
        reserve_msat: u64,
    ) -> Result<Payment, ApplicationError>;

    async fn settle_outgoing_payment(
        &self,
        payment_id: Uuid,
        actual_fee_msat: u64,
        settlement: PaymentSettlement,
    ) -> Result<Payment, ApplicationError>;

    async fn fail_outgoing_payment(
        &self,
        payment_id: Uuid,
        reason: String,
    ) -> Result<Payment, ApplicationError>;

    async fn settle_internal_payment_new_invoice(
        &self,
        payment: Payment,
        receiver_invoice: Invoice,
    ) -> Result<Payment, ApplicationError>;

    async fn settle_internal_payment_existing_invoice(
        &self,
        payment: Payment,
        invoice_id: Uuid,
        settlement: InvoiceSettlement,
    ) -> Result<Payment, ApplicationError>;
}
```

Exact type names can change during implementation. The important design rule is that each method names one business-atomic operation.

The payment service still owns policy:

- parse and validate payment input;
- reject self-payments;
- decide internal vs external path;
- build `Payment` and `Invoice` domain objects;
- compute the reserve/debit amount from amount, known fee, or documented fee-reservation policy;
- call the correct Unit-of-Work operation;
- perform external Lightning/Bitcoin calls outside retried DB transactions.

The Unit-of-Work owns mechanics:

- start/commit/rollback DB transactions;
- execute balance reservations/debits/credits;
- classify retryable DB errors before stringifying them;
- execute conditional invoice state transitions;
- keep payment, invoice, and wallet balance rows consistent.

Use a separate `EventProjectionUnitOfWork` for onchain event projection. Do not grow one global Unit-of-Work trait into another god object.

### 5. Introduce a wallet balance/reservation model

Add a materialized balance table, for example:

```text
wallet_balance
- wallet_id UUID primary key, foreign key to wallet(id)
- available_msat BIGINT NOT NULL
- reserved_msat BIGINT NOT NULL DEFAULT 0
- created_at TIMESTAMP NOT NULL
- updated_at TIMESTAMP NULL
```

Also persist the reservation amount attached to a pending outgoing payment. The implementation can choose one of these shapes:

- add `reserved_msat` to `payment`; or
- add a dedicated `payment_reservation` / ledger-reservation table.

For 0.2.0, prefer the smallest schema that keeps accounting correct and auditable.

The core reserve operation is an atomic conditional update:

```sql
UPDATE wallet_balance
SET available_msat = available_msat - :reserve_msat,
    reserved_msat = reserved_msat + :reserve_msat
WHERE wallet_id = :wallet_id
  AND available_msat >= :reserve_msat;
```

The operation succeeds only when `rows_affected == 1`. If no row is updated, return `DataError::InsufficientFunds`.

#### Outgoing payment lifecycle

1. Before the external Lightning/Bitcoin call, reserve funds and insert a pending payment in one DB transaction.
2. If the external call fails, release the reservation and mark the payment failed in one DB transaction.
3. If the external call succeeds, settle the payment and adjust the reservation to the actual debit in one DB transaction:
   - release any unused reserve back to `available_msat`;
   - if actual fee exceeds reserve, attempt an additional conditional debit or fail according to the provider/payment policy;
   - set the real `fee_msat` on the payment.

#### Internal payment lifecycle

Internal payments do not need a pending reservation. They are immediate ledger moves:

- debit sender `available_msat` with an atomic conditional update;
- credit receiver `available_msat`;
- insert sender payment;
- insert or update receiver invoice;
- commit all changes together.

Existing Bolt11 invoice settlement must be conditional:

```sql
UPDATE invoice
SET status = 'Settled', ...
WHERE id = :invoice_id
  AND status = 'Pending';
```

If `rows_affected == 0`, treat it as already settled/expired/conflict. This prevents two concurrent payers from settling the same pending internal invoice.

#### Incoming payment lifecycle

When an invoice becomes settled from Lightning or onchain event handling, credit the receiver wallet in the same transaction as the invoice state update. Idempotency must be enforced so repeated events do not double-credit a wallet.

## Migration and backfill strategy

The migration should support both SQLite and Postgres.

1. Create the new balance/reservation table and any reservation column/table.
2. Create a balance row for every wallet.
3. Backfill from existing data:
   - `received_msat`: settled invoices with `amount_received_msat`;
   - `spent_msat`: settled payments amount plus known fee;
   - `reserved_msat`: pending outgoing payments amount plus known fee/reservation where available;
   - `available_msat = received_msat - spent_msat - reserved_msat`.
4. For existing pending payments without an explicit reserved amount, use the persisted amount and known fee if present. Do not silently apply the old `fee_buffer` during backfill unless the implementation explicitly documents that policy and persists the resulting reservation.
5. Add creation logic so every new wallet gets a balance row in the same transaction as the wallet row.
6. Add reconciliation tests comparing the new materialized balance with the old aggregate calculation on fixture data.

The migration should fail loudly if backfilled balances would be negative in a way the new model cannot represent. That indicates existing inconsistent data requiring manual reconciliation.

## Postgres and SQLite behavior

Postgres is the priority deployment for company/server use. The balance invariant should be enforced by atomic conditional updates, not by hoping transaction isolation catches aggregate-read races.

SQLite remains supported for personal/self-hosted deployments. The same conditional-update shape works on SQLite. Expected concurrency is lower, but tests should still cover insufficient funds and concurrent reservation attempts.

Do not rely on `begin_with_config(Some(IsolationLevel::Serializable), None)` as the primary correctness mechanism. SeaORM applies serializable isolation for Postgres, but SQLite does not support setting transaction isolation through that API. Retrying serializable transactions can still be useful for Postgres edge cases, but the core invariant should be the conditional balance update.

Retry classification must happen while raw database error codes are still available. If errors are first converted into string-only `DatabaseError` values, SQLSTATE and SQLite extended error codes are lost.

## Testing strategy

### Unit tests

Service unit tests should use generated `mockall` mocks and should not start real databases or Lightning/Bitcoin nodes.

After `AppStore` becomes pure, tests can build services with a `StoreMocks` helper:

```rust
#[cfg(test)]
pub struct StoreMocks {
    pub payment: MockPaymentRepository,
    pub wallet: MockWalletRepository,
    pub invoice: MockInvoiceRepository,
    pub payment_uow: MockPaymentUnitOfWork,
    pub health: MockHealthProbe,
    // ...
}
```

Unit tests should verify service policy and interactions:

- self-payment validation;
- internal vs external routing;
- reserve amount calculation;
- propagation of Unit-of-Work errors;
- no external call when reserve fails;
- failure path calls reservation-release UoW method.

### Integration tests

Database-backed integration tests are required for the accounting invariant.

SQLite tests should run locally and in CI. Postgres tests should run with a GitHub Actions service container or another standard reproducible setup.

Cover at least:

- insufficient funds cannot reserve/create outgoing payment;
- two concurrent outgoing payments cannot overdraw a wallet;
- failed external payment releases reservation;
- settled external payment adjusts reservation to actual fee;
- internal payment with a new receiver invoice is atomic;
- internal Bolt11 existing-invoice settlement cannot be double-settled;
- migration/backfill produces expected balances;
- repeated incoming-payment events do not double-credit.

## Implementation sequence

1. #236: make `AppStore` a pure trait container and move SeaORM store construction into infra.
2. #238: remove SeaORM transaction types from repository traits.
3. #237: add wallet balance/reservation schema and backfill.
4. #239: implement `PaymentUnitOfWork` and refactor payment flows onto it.
5. #240: add SQLite/Postgres integration and concurrency tests.
6. #241: add `EventProjectionUnitOfWork` for onchain deposit projections.

These can be separate PRs, but the payment-critical accounting pieces should land before cutting a production/company-facing release that depends on Postgres correctness.

## Consequences

### Benefits

- Services become mockable without test-only production seams.
- Domain/application ports no longer expose SeaORM transaction types.
- Transaction scope becomes explicit and hard to misuse.
- Postgres deployments get a real overdraft-prevention invariant.
- SQLite remains supported through the same conditional-update model.
- Fee reservation becomes explicit instead of hidden behind an ambiguous `fee_buffer` admission check.

### Costs

- Requires a database migration and backfill.
- Requires DB-backed integration tests, including Postgres CI setup.
- Requires touching payment lifecycle code beyond the initial insert path.
- Requires careful idempotency handling for incoming-payment credits and event replay.

## Open decisions for implementation

- Whether to remove `fee_buffer` entirely for 0.2.0 or replace it with a documented explicit fee-reservation policy.
- Whether reservation data belongs on `payment.reserved_msat` or in a separate reservation/ledger table.
- Whether to expose balance reconciliation/admin tooling in the first implementation PR or defer it.
- Whether Postgres integration tests should be mandatory in all PR CI or initially run in a separate workflow.
