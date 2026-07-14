//! Real-DB Unit-of-Work tests (#240): exercise the payment reservation/settlement
//! UoW and the wallet-balance invariants against a real sqlite/postgres, not mocks
//! (a mock DB cannot test transaction serialization or the conditional updates).
//!
//! Gated behind `itest` so they stay out of the fast mocked unit run. The DB is
//! provisioned from `SWISSKNIFE_ITEST_DATABASE`; run via `make test-persistence`.

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, EntityTrait,
    QueryFilter, Statement,
};
use uuid::Uuid;

use crate::application::composition::Ledger;
use crate::application::errors::{ApplicationError, DataError};
use crate::domains::account::{AccountFilter, AccountRepository, ApiKey, ApiKeyRepository, AuthProvider, Permission};
use crate::domains::event::EventProjectionUnitOfWork;
use crate::domains::invoice::{Invoice, InvoiceRepository};
use crate::domains::payment::{LnPayment, Payment, PaymentStatus, PaymentUnitOfWork};
use crate::domains::{asset::AssetRepository, bitcoin::BtcNetwork, wallet::WalletRepository};

use super::models::{prelude::Wallet, wallet};
use super::{
    SeaOrmAccountRepository, SeaOrmApiKeyRepository, SeaOrmAssetRepository, SeaOrmEventProjectionUnitOfWork,
    SeaOrmInvoiceRepository, SeaOrmPaymentUnitOfWork, SeaOrmWalletRepository,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Provision a fresh database (a sqlite file or a new postgres db) from
/// `SWISSKNIFE_ITEST_DATABASE` and return a connection.
async fn connect_unmigrated() -> DatabaseConnection {
    let kind = std::env::var("SWISSKNIFE_ITEST_DATABASE").unwrap_or_else(|_| "sqlite".to_string());
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);

    let (url, max_conn) = match kind.as_str() {
        "sqlite" => {
            let dir = std::path::Path::new("target/itest");
            std::fs::create_dir_all(dir).expect("create sqlite dir");
            let path = dir.join(format!("uow-{}-{n}.db", std::process::id()));
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
            }
            // A single connection serializes writes, so concurrent reservations
            // exercise the conditional UPDATE without tripping SQLITE_BUSY.
            (format!("sqlite://{}?mode=rwc", path.display()), 1)
        }
        "postgres" => {
            let admin_url = std::env::var("SWISSKNIFE_ITEST_POSTGRES_ADMIN_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/postgres".to_string());
            let base_url = std::env::var("SWISSKNIFE_ITEST_POSTGRES_BASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432".to_string());
            let name = format!("uow_{}_{n}", std::process::id());
            let admin = Database::connect(&admin_url).await.expect("connect postgres admin");
            for stmt in [
                format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"),
                format!("CREATE DATABASE \"{name}\""),
            ] {
                admin
                    .execute(Statement::from_string(DatabaseBackend::Postgres, stmt))
                    .await
                    .expect("provision postgres db");
            }
            (format!("{}/{name}", base_url.trim_end_matches('/')), 5)
        }
        other => panic!("unknown SWISSKNIFE_ITEST_DATABASE '{other}' (expected sqlite or postgres)"),
    };

    let mut opt = ConnectOptions::new(url);
    opt.max_connections(max_conn);
    Database::connect(opt).await.expect("connect test database")
}

async fn connect() -> DatabaseConnection {
    let conn = connect_unmigrated().await;
    Migrator::up(&conn, None).await.expect("run migrations");
    conn
}

/// Register a wallet and credit it `balance_msat` of available funds.
async fn seed_wallet(conn: &DatabaseConnection, balance_msat: u64) -> Uuid {
    let account = SeaOrmAccountRepository::new(conn.clone())
        .insert(None, &[])
        .await
        .expect("create account");
    let asset = SeaOrmAssetRepository::new(conn.clone())
        .find_native_btc_by_network(BtcNetwork::Bitcoin)
        .await
        .expect("find native BTC asset")
        .expect("native BTC asset");
    let wallet = SeaOrmWalletRepository::new(conn.clone())
        .upsert(account.id, asset.id)
        .await
        .expect("ensure wallet");
    if balance_msat > 0 {
        SeaOrmWalletRepository::new(conn.clone())
            .credit(wallet.id, balance_msat)
            .await
            .expect("credit balance");
    }
    wallet.id
}

/// `(available_msat, reserved_msat)` for a wallet's balance row.
async fn balance(conn: &DatabaseConnection, wallet_id: Uuid) -> (i64, i64) {
    let row = Wallet::find()
        .filter(wallet::Column::Id.eq(wallet_id))
        .one(conn)
        .await
        .expect("query balance");
    row.map(|r| (r.available_amount, r.reserved_amount)).unwrap_or((0, 0))
}

/// An outgoing Lightning payment with a unique payment hash.
fn pending_payment(wallet_id: Uuid, amount_msat: u64, fee_msat: u64) -> Payment {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    Payment {
        wallet_id,
        amount_msat,
        fee_msat: Some(fee_msat),
        ledger: Ledger::Lightning,
        lightning: Some(LnPayment {
            payment_hash: format!("ph-{}-{n}", std::process::id()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// A pending invoice for a receiver wallet (internal ledger, awaiting payment).
fn pending_invoice(wallet_id: Uuid, amount_msat: u64) -> Invoice {
    Invoice {
        wallet_id,
        amount_msat: Some(amount_msat),
        amount_received_msat: Some(amount_msat),
        ledger: Ledger::Internal,
        ..Default::default()
    }
}

fn uow(conn: &DatabaseConnection) -> SeaOrmPaymentUnitOfWork {
    SeaOrmPaymentUnitOfWork::new(conn.clone())
}

async fn count(conn: &DatabaseConnection, sql: &str) -> i64 {
    conn.query_one(Statement::from_string(conn.get_database_backend(), sql.to_string()))
        .await
        .expect("query count")
        .expect("count row")
        .try_get::<i64>("", "count")
        .expect("count value")
}

#[tokio::test]
async fn postgres_migrates_legacy_oauth2_wallet_data() {
    if std::env::var("SWISSKNIFE_ITEST_DATABASE").as_deref() != Ok("postgres") {
        return;
    }

    let conn = connect_unmigrated().await;
    Migrator::up(&conn, Some(16)).await.expect("run pre-account migrations");
    conn.execute_unprepared(
        r#"
        INSERT INTO wallet (id, user_id, created_at)
        VALUES ('11111111-1111-4111-8111-111111111111', 'auth0|alice', CURRENT_TIMESTAMP);

        INSERT INTO wallet_balance (
            wallet_id, currency, available_amount, reserved_amount, created_at
        ) VALUES (
            '11111111-1111-4111-8111-111111111111', 'Bitcoin', 12345, 678, CURRENT_TIMESTAMP
        );

        INSERT INTO ln_address (
            id, wallet_id, username, active, allows_nostr, created_at
        ) VALUES (
            '44444444-4444-4444-8444-444444444444',
            '11111111-1111-4111-8111-111111111111',
            'alice',
            TRUE,
            FALSE,
            CURRENT_TIMESTAMP
        );
        "#,
    )
    .await
    .expect("insert legacy production-shaped data");

    Migrator::up(&conn, None).await.expect("run account cutover");

    assert_eq!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM wallet
            JOIN auth_identity ON auth_identity.account_id = wallet.account_id
            JOIN asset ON asset.id = wallet.asset_id
            WHERE wallet.id = '11111111-1111-4111-8111-111111111111'
              AND auth_identity.provider = 'oauth2'
              AND auth_identity.subject = 'auth0|alice'
              AND asset.protocol = 'bitcoin'
              AND asset.network = 'Bitcoin'
              AND asset.asset_ref = 'native'
              AND wallet.available_amount = 12345
              AND wallet.reserved_amount = 678
            "#,
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM ln_address
            JOIN wallet ON wallet.id = ln_address.wallet_id
            WHERE ln_address.account_id = wallet.account_id
            "#,
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name IN ('wallet', 'api_key')
              AND column_name = 'user_id'
            "#,
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM pg_indexes
            WHERE schemaname = 'public'
              AND indexname IN (
                'idx_api_key_account_id',
                'idx_asset_protocol_network_ref',
                'idx_auth_identity_account_id',
                'idx_auth_identity_provider_subject',
                'idx_btc_address_wallet_used',
                'idx_btc_output_txid_output_index',
                'idx_invoice_btc_output_id',
                'idx_invoice_ln_address_id',
                'idx_invoice_wallet_created_at',
                'idx_ln_address_account',
                'idx_payment_wallet_created_at',
                'idx_wallet_account_asset',
                'idx_wallet_account_id',
                'idx_wallet_asset_id'
              )
            "#,
        )
        .await,
        14
    );
}

#[tokio::test]
async fn account_identity_upsert_is_idempotent() {
    let conn = connect().await;
    let repo = SeaOrmAccountRepository::new(conn.clone());

    let first = repo.upsert(AuthProvider::Jwt, "alice", None, &[]).await.unwrap();
    let second = repo.upsert(AuthProvider::Jwt, "alice", None, &[]).await.unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(
        first.identity.as_ref().map(|identity| identity.subject.as_str()),
        Some("alice")
    );
    assert_eq!(first.preferences.as_ref().map(|_| ()), Some(()));
    assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM account").await, 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM auth_identity").await, 1);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) AS count FROM account_preference").await,
        1
    );
}

#[tokio::test]
async fn account_identity_upsert_inserts_initial_permissions() {
    let conn = connect().await;
    let repo = SeaOrmAccountRepository::new(conn.clone());

    let account = repo
        .upsert(
            AuthProvider::Jwt,
            "alice",
            None,
            &[Permission::ReadWallet, Permission::ReadWallet],
        )
        .await
        .unwrap();

    assert_eq!(account.permissions, Some(vec![Permission::ReadWallet]));
    assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM account").await, 1);
}

#[tokio::test]
async fn account_repository_lists_accounts_with_and_without_identities() {
    let conn = connect().await;
    let repo = SeaOrmAccountRepository::new(conn);
    let managed = repo.insert(Some("Managed".to_string()), &[]).await.unwrap();
    let login = repo.upsert(AuthProvider::Jwt, "alice", None, &[]).await.unwrap();

    let accounts = repo
        .find_many(AccountFilter {
            ids: Some(vec![managed.id, login.id]),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(accounts.len(), 2);
    assert!(accounts
        .iter()
        .find(|account| account.id == managed.id)
        .unwrap()
        .identity
        .is_none());
    assert_eq!(
        accounts
            .iter()
            .find(|account| account.id == login.id)
            .unwrap()
            .identity
            .as_ref()
            .map(|identity| identity.subject.as_str()),
        Some("alice")
    );
}

#[tokio::test]
async fn account_repository_crud_keeps_the_aggregate_consistent() {
    let conn = connect().await;
    let repo = SeaOrmAccountRepository::new(conn.clone());
    let account = repo
        .insert(Some("Operator".to_string()), &[Permission::ReadWallet])
        .await
        .unwrap();
    assert!(account.identity.is_none());

    let listed = repo
        .find_many(AccountFilter {
            ids: Some(vec![account.id]),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].display_name.as_deref(), Some("Operator"));

    let mut updated = account.clone();
    updated.display_name = Some("Treasury".to_string());
    let updated = repo.update(updated).await.unwrap();
    assert_eq!(updated.display_name.as_deref(), Some("Treasury"));

    let mut updated = updated;
    updated.permissions = Some(vec![Permission::ReadAccount, Permission::WriteWallet]);
    let updated = repo.update(updated).await.unwrap();
    assert_eq!(
        updated.permissions,
        Some(vec![Permission::ReadAccount, Permission::WriteWallet])
    );

    let asset = SeaOrmAssetRepository::new(conn.clone())
        .find_native_btc_by_network(BtcNetwork::Bitcoin)
        .await
        .unwrap()
        .unwrap();
    let wallet_repo = SeaOrmWalletRepository::new(conn.clone());
    let wallet = wallet_repo.upsert(account.id, asset.id).await.unwrap();
    assert!(wallet_repo.exists_for_account(account.id, wallet.id).await.unwrap());
    assert!(!wallet_repo.exists_for_account(Uuid::new_v4(), wallet.id).await.unwrap());
    SeaOrmApiKeyRepository::new(conn.clone())
        .insert(ApiKey {
            account_id: account.id,
            name: "operator key".to_string(),
            key_hash: vec![42; 32],
            permissions: vec![Permission::ReadWallet],
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(
        repo.delete_many(AccountFilter {
            ids: Some(vec![account.id]),
            ..Default::default()
        })
        .await
        .unwrap(),
        1
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM account").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM auth_identity").await, 0);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) AS count FROM account_preference").await,
        0
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM wallet").await, 0);
    assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM api_key").await, 0);
}

#[tokio::test]
async fn reserve_rejects_insufficient_funds() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 50_000).await;

    let err = uow(&conn)
        .reserve(pending_payment(wallet, 100_000, 0), 100_000)
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Data(DataError::InsufficientFunds(_))));
    // The reservation rolled back: nothing held, balance untouched.
    assert_eq!(balance(&conn, wallet).await, (50_000, 0));
}

#[tokio::test]
async fn reserve_holds_funds() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 200_000).await;

    let payment = uow(&conn)
        .reserve(pending_payment(wallet, 100_000, 0), 110_000)
        .await
        .expect("reserve");

    assert_eq!(payment.status, PaymentStatus::Pending);
    assert_eq!(payment.reserved_amount, 110_000);
    assert_eq!(balance(&conn, wallet).await, (90_000, 110_000));
}

#[tokio::test]
async fn concurrent_reserves_cannot_overdraw() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 150_000).await;

    // Two 100k reservations against 150k: only one can fit.
    let u1 = uow(&conn);
    let u2 = uow(&conn);
    let (r1, r2) = tokio::join!(
        u1.reserve(pending_payment(wallet, 100_000, 0), 100_000),
        u2.reserve(pending_payment(wallet, 100_000, 0), 100_000),
    );

    let succeeded = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    assert_eq!(succeeded, 1, "exactly one reservation may succeed");
    assert_eq!(balance(&conn, wallet).await, (50_000, 100_000));
}

#[tokio::test]
async fn fail_releases_the_reservation() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 200_000).await;
    let mut payment = uow(&conn)
        .reserve(pending_payment(wallet, 100_000, 0), 110_000)
        .await
        .expect("reserve");
    payment.status = PaymentStatus::Failed;

    let failed = uow(&conn).fail(payment).await.expect("fail");

    assert_eq!(failed.status, PaymentStatus::Failed);
    // The full reservation returns to available; nothing is debited.
    assert_eq!(balance(&conn, wallet).await, (200_000, 0));
}

#[tokio::test]
async fn duplicate_fail_does_not_double_release() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 200_000).await;
    let mut payment = uow(&conn)
        .reserve(pending_payment(wallet, 100_000, 0), 110_000)
        .await
        .expect("reserve");
    payment.status = PaymentStatus::Failed;
    payment.error = Some("payment failed".to_string());

    let first = uow(&conn).fail(payment.clone()).await.expect("first fail");
    assert_eq!(first.status, PaymentStatus::Failed);
    assert_eq!(balance(&conn, wallet).await, (200_000, 0));

    // The failure event and replay sync can both observe the same failed node
    // payment. The replay must return the already-failed row without trying to
    // release the stale reservation again.
    let second = uow(&conn).fail(payment).await.expect("second fail");
    assert_eq!(second.status, PaymentStatus::Failed);
    assert_eq!(balance(&conn, wallet).await, (200_000, 0));
}

#[tokio::test]
async fn settle_adjusts_reserved_to_actual() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 200_000).await;
    // Reserve with headroom (110k) for an unknown routing fee.
    let mut payment = uow(&conn)
        .reserve(pending_payment(wallet, 100_000, 1_000), 110_000)
        .await
        .expect("reserve");
    assert_eq!(balance(&conn, wallet).await, (90_000, 110_000));

    // Settles for the actual amount + fee (101k), not the reserved 110k.
    payment.status = PaymentStatus::Settled;
    let settled = uow(&conn).settle(payment).await.expect("settle");

    assert_eq!(settled.status, PaymentStatus::Settled);
    // The 110k reservation is released and exactly 101k debited; the 9k headroom
    // returns to available.
    assert_eq!(balance(&conn, wallet).await, (99_000, 0));
}

#[tokio::test]
async fn settle_records_confirmed_spend_when_actual_exceeds_reserved() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 110_000).await;
    // The node ultimately reports a 20k fee, exceeding the 10k admission buffer.
    let mut payment = uow(&conn)
        .reserve(pending_payment(wallet, 100_000, 20_000), 110_000)
        .await
        .expect("reserve");
    assert_eq!(balance(&conn, wallet).await, (0, 110_000));

    payment.status = PaymentStatus::Settled;
    let settled = uow(&conn)
        .settle(payment)
        .await
        .expect("confirmed settlement must not be stranded");

    assert_eq!(settled.status, PaymentStatus::Settled);
    assert_eq!(settled.reserved_amount, 0);
    // The ledger records the confirmed 120k spend even though only 110k was held,
    // surfacing the overspend as a negative available balance for reconciliation.
    assert_eq!(balance(&conn, wallet).await, (-10_000, 0));
}

#[tokio::test]
async fn duplicate_settle_does_not_double_debit() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 200_000).await;
    let mut payment = uow(&conn)
        .reserve(pending_payment(wallet, 100_000, 1_000), 110_000)
        .await
        .expect("reserve");
    payment.status = PaymentStatus::Settled;

    let first = uow(&conn).settle(payment.clone()).await.expect("first settle");
    assert_eq!(first.status, PaymentStatus::Settled);
    assert_eq!(balance(&conn, wallet).await, (99_000, 0));

    // The success event and the sync result both settle: the loser is a no-op,
    // returning the already-settled payment without debiting again.
    let second = uow(&conn).settle(payment).await.expect("second settle");
    assert_eq!(second.status, PaymentStatus::Settled);
    assert_eq!(balance(&conn, wallet).await, (99_000, 0));
}

#[tokio::test]
async fn fail_then_settle_corrects_without_double_release() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 200_000).await;
    let reserved = uow(&conn)
        .reserve(pending_payment(wallet, 100_000, 1_000), 110_000)
        .await
        .expect("reserve");

    // A premature error marks it failed and releases the reservation.
    let mut to_fail = reserved.clone();
    to_fail.status = PaymentStatus::Failed;
    to_fail.error = Some("premature RPC timeout".to_string());
    uow(&conn).fail(to_fail).await.expect("fail");
    assert_eq!(balance(&conn, wallet).await, (200_000, 0));

    // The delayed success event settles with the *stale* payload (still carrying
    // reserved_amount = 110k). The UoW must read the stored reservation (now 0)
    // rather than the stale amount, settle the payment, debit the actual, and
    // clear the failure reason.
    let mut to_settle = reserved;
    to_settle.status = PaymentStatus::Settled;
    let settled = uow(&conn).settle(to_settle).await.expect("settle corrects the failure");

    assert_eq!(settled.status, PaymentStatus::Settled);
    assert!(settled.error.is_none(), "a settled payment carries no failure reason");
    assert_eq!(balance(&conn, wallet).await, (99_000, 0));
}

#[tokio::test]
async fn settle_internal_is_atomic() {
    let conn = connect().await;
    let payer = seed_wallet(&conn, 200_000).await;
    let payee = seed_wallet(&conn, 0).await;

    // A brand-new receiver invoice (nil id) is created in the same transaction.
    let mut payment = pending_payment(payer, 50_000, 0);
    payment.status = PaymentStatus::Settled;
    let mut invoice = pending_invoice(payee, 50_000);
    invoice.payment_time = Some(Utc::now());
    let settled = uow(&conn)
        .settle_internal(payment, invoice)
        .await
        .expect("settle_internal");

    assert_eq!(settled.status, PaymentStatus::Settled);
    assert_eq!(balance(&conn, payer).await, (150_000, 0), "payer debited");
    assert_eq!(balance(&conn, payee).await, (50_000, 0), "payee credited");
}

#[tokio::test]
async fn concurrent_internal_payers_cannot_both_settle_one_invoice() {
    let conn = connect().await;
    let payer_a = seed_wallet(&conn, 100_000).await;
    let payer_b = seed_wallet(&conn, 100_000).await;
    let payee = seed_wallet(&conn, 0).await;

    // One pending invoice the payee issued.
    let mut invoice = SeaOrmInvoiceRepository::new(conn.clone())
        .insert(pending_invoice(payee, 50_000))
        .await
        .expect("insert invoice");
    // The settlement carries the payment time; the conditional pending->settled
    // update must mark it paid exactly once across the two payers.
    invoice.payment_time = Some(Utc::now());

    let mut pa = pending_payment(payer_a, 50_000, 0);
    pa.status = PaymentStatus::Settled;
    let mut pb = pending_payment(payer_b, 50_000, 0);
    pb.status = PaymentStatus::Settled;

    let u1 = uow(&conn);
    let u2 = uow(&conn);
    let (r1, r2) = tokio::join!(u1.settle_internal(pa, invoice.clone()), u2.settle_internal(pb, invoice));

    let succeeded = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    assert_eq!(succeeded, 1, "only one payer settles the invoice");
    let err = [r1, r2]
        .into_iter()
        .find_map(Result::err)
        .expect("the other payer conflicts");
    assert!(matches!(err, ApplicationError::Data(DataError::Conflict(_))));

    // The payee is credited exactly once, and exactly one payer is debited.
    assert_eq!(balance(&conn, payee).await, (50_000, 0));
    let (a, b) = (balance(&conn, payer_a).await.0, balance(&conn, payer_b).await.0);
    assert!(
        (a == 50_000 && b == 100_000) || (a == 100_000 && b == 50_000),
        "exactly one payer debited (a={a}, b={b})"
    );
}

#[tokio::test]
async fn settle_incoming_invoice_credits_a_new_invoice() {
    let conn = connect().await;
    let receiver = seed_wallet(&conn, 0).await;

    // A confirmed incoming invoice first seen settled (nil id) credits the receiver.
    let mut invoice = pending_invoice(receiver, 30_000);
    invoice.payment_time = Some(Utc::now());
    SeaOrmEventProjectionUnitOfWork::new(conn.clone())
        .settle_incoming_invoice(invoice)
        .await
        .expect("settle incoming");

    assert_eq!(balance(&conn, receiver).await, (30_000, 0));
}

#[tokio::test]
async fn settle_incoming_invoice_credits_once_under_replay() {
    let conn = connect().await;
    let receiver = seed_wallet(&conn, 0).await;
    let mut invoice = SeaOrmInvoiceRepository::new(conn.clone())
        .insert(pending_invoice(receiver, 30_000))
        .await
        .expect("insert pending invoice");
    invoice.payment_time = Some(Utc::now());

    let projection = SeaOrmEventProjectionUnitOfWork::new(conn.clone());
    projection
        .settle_incoming_invoice(invoice.clone())
        .await
        .expect("settle");
    assert_eq!(balance(&conn, receiver).await, (30_000, 0), "credited on settlement");

    // A duplicate success event must not credit twice.
    projection
        .settle_incoming_invoice(invoice)
        .await
        .expect("idempotent replay");
    assert_eq!(
        balance(&conn, receiver).await,
        (30_000, 0),
        "no double credit on replay"
    );
}
