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
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection,
    EntityTrait, QueryFilter, Set, Statement, TransactionTrait,
};
use uuid::Uuid;

use crate::application::composition::Ledger;
use crate::application::errors::{ApplicationError, DataError};
use crate::domains::account::{AccountFilter, AccountRepository, ApiKey, ApiKeyRepository, AuthProvider, Permission};
use crate::domains::event::{ClientEventRepository, ClientEventType, EventProjectionUnitOfWork};
use crate::domains::invoice::{Invoice, InvoiceRepository};
use crate::domains::ln_address::LnAddressRepository;
use crate::domains::payment::{LnPayment, Payment, PaymentRepository, PaymentStatus, PaymentUnitOfWork};
use crate::domains::{
    asset::AssetRepository,
    bitcoin::{BtcAddressRepository, BtcAddressType, BtcNetwork, BtcOutput, BtcOutputStatus},
    wallet::WalletRepository,
};

use super::models::{
    client_event,
    prelude::{ClientEvent as ClientEventEntity, Wallet},
    wallet,
};
use super::{
    NewClientEvent, SeaOrmAccountRepository, SeaOrmApiKeyRepository, SeaOrmAssetRepository,
    SeaOrmBitcoinAddressRepository, SeaOrmClientEventRepository, SeaOrmEventProjectionUnitOfWork,
    SeaOrmInvoiceRepository, SeaOrmLnAddressRepository, SeaOrmPaymentRepository, SeaOrmPaymentUnitOfWork,
    SeaOrmWalletRepository,
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
                    .execute_raw(Statement::from_string(DatabaseBackend::Postgres, stmt))
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

async fn account_id(conn: &DatabaseConnection, wallet_id: Uuid) -> Uuid {
    Wallet::find_by_id(wallet_id)
        .one(conn)
        .await
        .expect("query wallet")
        .expect("wallet exists")
        .account_id
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
    conn.query_one_raw(Statement::from_string(conn.get_database_backend(), sql.to_string()))
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
                'idx_client_event_created_at',
                'idx_client_event_type_resource',
                'idx_client_event_wallet_id',
                'idx_invoice_btc_output_id',
                'idx_invoice_ln_address_id',
                'idx_invoice_wallet_created_at',
                'idx_ln_address_account',
                'idx_payment_wallet_created_at',
                'idx_wallet_account_asset',
                'uq_wallet_account_id',
                'idx_wallet_asset_id'
              )
            "#,
        )
        .await,
        17
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
    wallet_repo.credit(wallet.id, 42_000).await.unwrap();
    let mut invoice = pending_invoice(wallet.id, 42_000);
    invoice.payment_time = Some(Utc::now());
    SeaOrmInvoiceRepository::new(conn.clone())
        .insert(invoice)
        .await
        .unwrap();
    let mut payment = pending_payment(wallet.id, 10_000, 250);
    payment.status = PaymentStatus::Settled;
    payment.payment_time = Some(Utc::now());
    SeaOrmPaymentRepository::new(conn.clone())
        .insert(payment)
        .await
        .unwrap();
    assert!(wallet_repo.exists_for_account(account.id, wallet.id).await.unwrap());
    assert!(!wallet_repo.exists_for_account(Uuid::new_v4(), wallet.id).await.unwrap());
    let ln_address = SeaOrmLnAddressRepository::new(conn.clone())
        .insert(account.id, wallet.id, "operator", false, None)
        .await
        .unwrap();
    let aggregate = repo.find(account.id).await.unwrap().unwrap();
    assert!(aggregate.identity.is_none());
    assert_eq!(aggregate.wallets.len(), 1);
    assert_eq!(aggregate.wallets[0].id, wallet.id);
    assert_eq!(aggregate.wallets[0].balance.available_msat, 42_000);
    assert_eq!(aggregate.wallets[0].balance.received_msat, 42_000);
    assert_eq!(aggregate.wallets[0].balance.sent_msat, 10_000);
    assert_eq!(aggregate.wallets[0].balance.fees_paid_msat, 250);
    assert_eq!(
        aggregate.wallets[0].asset.as_ref().map(|asset| asset.id),
        Some(asset.id)
    );
    assert_eq!(
        aggregate.wallets[0].ln_address.as_ref().map(|address| address.id),
        Some(ln_address.id)
    );
    assert!(aggregate.wallets[0].payments.is_empty());
    assert!(aggregate.wallets[0].invoices.is_empty());
    assert!(aggregate.wallets[0].btc_addresses.is_empty());
    assert!(aggregate.wallets[0].contacts.is_empty());
    let listed = repo
        .find_many(AccountFilter {
            ids: Some(vec![account.id]),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(listed[0].wallets[0].balance.received_msat, 42_000);
    assert_eq!(listed[0].wallets[0].balance.sent_msat, 10_000);
    assert_eq!(listed[0].wallets[0].balance.fees_paid_msat, 250);
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

    let events = SeaOrmClientEventRepository::new(conn.clone())
        .find_after(account_id(&conn, wallet).await, 0, 10)
        .await
        .expect("read events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, ClientEventType::PaymentFailed);
    assert_eq!(events[0].resource_id, first.id);
}

#[tokio::test]
async fn client_events_are_account_scoped_and_pruned_with_a_durable_watermark() {
    let conn = connect().await;
    let first_wallet = seed_wallet(&conn, 200_000).await;
    let second_wallet = seed_wallet(&conn, 200_000).await;
    let third_wallet = seed_wallet(&conn, 200_000).await;

    for wallet in [first_wallet, second_wallet, third_wallet] {
        let mut payment = uow(&conn)
            .reserve(pending_payment(wallet, 10_000, 0), 10_000)
            .await
            .expect("reserve payment");
        payment.status = PaymentStatus::Failed;
        uow(&conn).fail(payment).await.expect("fail payment");
    }

    let first_account = account_id(&conn, first_wallet).await;
    let second_account = account_id(&conn, second_wallet).await;
    let third_account = account_id(&conn, third_wallet).await;
    let repo = SeaOrmClientEventRepository::new(conn.clone());
    let first_events = repo.find_after(first_account, 0, 10).await.expect("first events");
    let second_events = repo.find_after(second_account, 0, 10).await.expect("second events");
    let third_events = repo.find_after(third_account, 0, 10).await.expect("third events");
    assert_eq!(first_events.len(), 1);
    assert_eq!(first_events[0].wallet_id, first_wallet);
    assert_eq!(second_events.len(), 1);
    assert_eq!(second_events[0].wallet_id, second_wallet);
    assert_eq!(third_events.len(), 1);
    assert_eq!(third_events[0].wallet_id, third_wallet);

    let notifications = super::SeaOrmNotificationRepository::new(conn.clone());
    use crate::domains::notification::NotificationRepository;
    assert_eq!(notifications.unread_count(first_account).await.unwrap(), 1);
    assert_eq!(notifications.unread_count(third_account).await.unwrap(), 1);

    let first_id = first_events[0].id.parse::<i32>().expect("numeric event ID");
    let second_id = second_events[0].id.parse::<i32>().expect("numeric event ID");
    for (event_id, age) in [(first_id, 3), (second_id, 2)] {
        let model = ClientEventEntity::find_by_id(event_id)
            .one(&conn)
            .await
            .expect("query event")
            .expect("event exists");
        let mut model: client_event::ActiveModel = model.into();
        model.created_at = Set((Utc::now() - chrono::Duration::days(age)).naive_utc());
        model.update(&conn).await.expect("age event");
    }

    let older_only = repo.clone();
    let all_expired = repo.clone();
    let (older_count, all_count) = tokio::join!(
        older_only.prune_before(Utc::now() - chrono::Duration::hours(60)),
        all_expired.prune_before(Utc::now() - chrono::Duration::days(1)),
    );
    assert_eq!(
        older_count.expect("prune oldest events") + all_count.expect("prune all expired events"),
        2
    );
    assert_eq!(notifications.unread_count(first_account).await.unwrap(), 0);
    assert_eq!(notifications.unread_count(second_account).await.unwrap(), 0);
    assert_eq!(notifications.unread_count(third_account).await.unwrap(), 1);
    assert_eq!(repo.pruned_through().await.expect("read prune watermark"), second_id);
    assert!(repo
        .find_after(first_account, 0, 10)
        .await
        .expect("first account after prune")
        .is_empty());
    assert!(repo
        .find_after(second_account, 0, 10)
        .await
        .expect("second account after prune")
        .is_empty());
    assert_eq!(
        repo.find_after(third_account, 0, 10)
            .await
            .expect("third account after prune")
            .len(),
        1,
        "retention must leave newer events intact"
    );
}

#[tokio::test]
async fn notification_migration_projects_existing_terminal_events() {
    use crate::domains::notification::NotificationRepository;

    let conn = connect().await;
    let owner_wallet = seed_wallet(&conn, 0).await;
    let unpaid = SeaOrmInvoiceRepository::new(conn.clone())
        .insert(pending_invoice(owner_wallet, 42_000))
        .await
        .unwrap();
    let txn = conn.begin().await.unwrap();
    SeaOrmClientEventRepository::new(&txn)
        .append_event(NewClientEvent::invoice_pending(&unpaid))
        .await
        .unwrap();
    let mut paid = unpaid;
    paid.payment_time = Some(Utc::now());
    paid.amount_received_msat = Some(42_000);
    SeaOrmClientEventRepository::new(&txn)
        .append_event(NewClientEvent::invoice_paid(&paid))
        .await
        .unwrap();
    txn.commit().await.unwrap();

    let migration = Migrator::migrations()
        .into_iter()
        .find(|m| m.name().ends_with("_account_notification"))
        .unwrap();
    let manager = migration::SchemaManager::new(&conn);
    migration.down(&manager).await.unwrap();
    migration.up(&manager).await.unwrap();
    let repo = super::SeaOrmNotificationRepository::new(conn.clone());
    let items = repo
        .list(account_id(&conn, owner_wallet).await, None, 10, false)
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].resource_id, paid.id);
    assert_eq!(items[0].event_type, ClientEventType::InvoicePaid);
    assert_eq!(items[0].amount_msat.as_deref(), Some("42000"));
}

#[tokio::test]
async fn concurrent_event_appends_preserve_commit_order() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 0).await;
    let account = account_id(&conn, wallet).await;
    let first_resource = Uuid::new_v4();
    let second_resource = Uuid::new_v4();
    let event = |resource_id| NewClientEvent {
        wallet_id: wallet,
        resource_id,
        event_type: ClientEventType::PaymentFailed,
        data: serde_json::json!({}),
    };
    let first = conn.begin().await.unwrap();
    SeaOrmClientEventRepository::new(&first)
        .append_event(event(first_resource))
        .await
        .unwrap();
    let second_event = event(second_resource);
    let second_conn = conn.clone();
    let mut second = tokio::spawn(async move {
        let txn = second_conn.begin().await.unwrap();
        SeaOrmClientEventRepository::new(&txn)
            .append_event(second_event)
            .await
            .unwrap();
        txn.commit().await.unwrap();
    });

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut second)
            .await
            .is_err(),
        "a later event must not commit before the earlier cursor"
    );
    first.commit().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), second)
        .await
        .unwrap()
        .unwrap();
    let events = SeaOrmClientEventRepository::new(conn)
        .find_after(account, 0, 10)
        .await
        .unwrap();
    assert_eq!(
        events.iter().map(|event| event.resource_id).collect::<Vec<_>>(),
        vec![first_resource, second_resource]
    );
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

    let events = SeaOrmClientEventRepository::new(conn.clone())
        .find_after(account_id(&conn, wallet).await, 0, 10)
        .await
        .expect("read events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, ClientEventType::PaymentSettled);
    assert_eq!(events[0].resource_id, first.id);
    assert_eq!(events[0].data["status"], "Settled");
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
async fn wallet_operations_reject_unsigned_amounts_outside_storage_range() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 1_000).await;
    let repo = SeaOrmWalletRepository::new(conn.clone());

    for amount in [i64::MAX as u64 + 1, u64::MAX - 999, u64::MAX] {
        assert!(repo.credit(wallet, amount).await.is_err());
        assert!(repo.debit(wallet, amount).await.is_err());
        assert!(repo.debit_confirmed(wallet, amount).await.is_err());
        assert!(repo.reserve(wallet, amount).await.is_err());
        assert!(repo.release(wallet, amount).await.is_err());
        assert_eq!(balance(&conn, wallet).await, (1_000, 0));
    }
}

#[tokio::test]
async fn wallet_operations_preserve_the_largest_supported_amount() {
    let conn = connect().await;
    let maximum = i64::MAX as u64;
    let wallet = seed_wallet(&conn, maximum).await;
    let repo = SeaOrmWalletRepository::new(conn.clone());

    assert!(repo.credit(wallet, 1).await.is_err());
    assert_eq!(balance(&conn, wallet).await, (i64::MAX, 0));
    assert!(repo.reserve(wallet, maximum).await.unwrap());
    assert_eq!(balance(&conn, wallet).await, (0, i64::MAX));
    assert!(repo.release(wallet, maximum).await.unwrap());
    assert_eq!(balance(&conn, wallet).await, (i64::MAX, 0));
    assert!(repo.debit(wallet, maximum).await.unwrap());
    assert_eq!(balance(&conn, wallet).await, (0, 0));
}

#[tokio::test]
async fn confirmed_debits_can_go_negative_without_underflowing() {
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 0).await;
    let repo = SeaOrmWalletRepository::new(conn.clone());

    assert!(repo.debit_confirmed(wallet, i64::MAX as u64).await.unwrap());
    assert!(repo.debit_confirmed(wallet, 1).await.unwrap());
    assert_eq!(balance(&conn, wallet).await, (i64::MIN, 0));
    assert!(!repo.debit_confirmed(wallet, 1).await.unwrap());
    assert_eq!(balance(&conn, wallet).await, (i64::MIN, 0));
}

#[tokio::test]
async fn oversized_internal_amount_cannot_create_spendable_balance() {
    let conn = connect().await;
    let payer = seed_wallet(&conn, 0).await;
    let payee = seed_wallet(&conn, 0).await;
    // This unsigned amount would become -1,000 in signed database storage.
    let amount = u64::MAX - 999;
    let mut payment = pending_payment(payer, amount, 0);
    payment.ledger = Ledger::Internal;
    payment.status = PaymentStatus::Settled;
    let mut invoice = pending_invoice(payee, amount);
    invoice.payment_time = Some(Utc::now());

    let result = uow(&conn).settle_internal(payment, invoice).await;

    assert!(result.is_err(), "an unrepresentable internal transfer must be rejected");
    assert_eq!(balance(&conn, payer).await, (0, 0));
    assert_eq!(balance(&conn, payee).await, (0, 0));
    let events = SeaOrmClientEventRepository::new(conn.clone())
        .find_after(account_id(&conn, payer).await, 0, 10)
        .await
        .expect("read payer events");
    assert!(events.is_empty(), "a rejected transfer emits no settlement event");
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

    let event_repo = SeaOrmClientEventRepository::new(conn.clone());
    let payer_events = event_repo
        .find_after(account_id(&conn, payer).await, 0, 10)
        .await
        .expect("payer events");
    let payee_events = event_repo
        .find_after(account_id(&conn, payee).await, 0, 10)
        .await
        .expect("payee events");
    assert_eq!(payer_events.len(), 1);
    assert_eq!(payer_events[0].event_type, ClientEventType::PaymentSettled);
    assert_eq!(payee_events.len(), 1);
    assert_eq!(payee_events[0].event_type, ClientEventType::InvoicePaid);
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
    let (first, duplicate) = tokio::join!(
        projection.settle_incoming_invoice(invoice.clone()),
        projection.settle_incoming_invoice(invoice.clone())
    );
    first.expect("first settlement");
    duplicate.expect("concurrent replay");
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

    use crate::domains::notification::NotificationRepository;
    let notices = super::SeaOrmNotificationRepository::new(conn.clone());
    let notices = notices
        .list(account_id(&conn, receiver).await, None, 10, false)
        .await
        .unwrap();
    assert_eq!(notices.len(), 1, "event replay must preserve one inbox item");
    let events = SeaOrmClientEventRepository::new(conn.clone())
        .find_after(account_id(&conn, receiver).await, 0, 10)
        .await
        .expect("read events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, ClientEventType::InvoicePaid);
}

#[tokio::test]
async fn onchain_deposit_emits_pending_then_paid_once() {
    let conn = connect().await;
    let receiver = seed_wallet(&conn, 0).await;
    let address = SeaOrmBitcoinAddressRepository::new(conn.clone())
        .insert(receiver, "bcrt1qeventdeposit", BtcAddressType::P2wpkh)
        .await
        .expect("insert deposit address");
    let projection = SeaOrmEventProjectionUnitOfWork::new(conn.clone());
    let unconfirmed_output = BtcOutput {
        outpoint: "event-deposit-tx:0".to_string(),
        txid: "event-deposit-tx".to_string(),
        address: address.address.clone(),
        amount_sat: 30,
        status: BtcOutputStatus::Unconfirmed,
        ..Default::default()
    };
    let pending = Invoice {
        wallet_id: receiver,
        amount_msat: Some(30_000),
        ledger: Ledger::Onchain,
        description: Some("Bitcoin On-chain deposit".to_string()),
        ..Default::default()
    };

    let stored = projection
        .project_onchain_deposit(unconfirmed_output.clone(), address.clone(), pending.clone())
        .await
        .expect("project pending deposit");
    assert_eq!(stored.status, crate::domains::invoice::InvoiceStatus::Pending);
    assert_eq!(balance(&conn, receiver).await, (0, 0));

    // Replaying the same mempool observation updates the projection without
    // emitting a duplicate pending event.
    projection
        .project_onchain_deposit(unconfirmed_output.clone(), address.clone(), pending.clone())
        .await
        .expect("replay pending deposit");

    let mut confirmed_output = unconfirmed_output;
    confirmed_output.status = BtcOutputStatus::Confirmed;
    confirmed_output.block_height = Some(42);
    let mut confirmed = pending;
    confirmed.amount_received_msat = Some(30_000);
    confirmed.payment_time = Some(Utc::now());
    let settled = projection
        .project_onchain_deposit(confirmed_output.clone(), address.clone(), confirmed.clone())
        .await
        .expect("confirm deposit");
    assert_eq!(settled.status, crate::domains::invoice::InvoiceStatus::Settled);
    assert_eq!(balance(&conn, receiver).await, (30_000, 0));

    // A confirmed replay neither credits nor emits twice.
    projection
        .project_onchain_deposit(confirmed_output, address, confirmed)
        .await
        .expect("replay confirmed deposit");
    assert_eq!(balance(&conn, receiver).await, (30_000, 0));

    let events = SeaOrmClientEventRepository::new(conn.clone())
        .find_after(account_id(&conn, receiver).await, 0, 10)
        .await
        .expect("read events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, ClientEventType::InvoicePending);
    assert_eq!(events[0].resource_id, stored.id);
    assert_eq!(events[0].data["status"], "Pending");
    assert_eq!(events[0].data["bitcoin_output"]["status"], "Unconfirmed");
    assert_eq!(events[1].event_type, ClientEventType::InvoicePaid);
    assert_eq!(events[1].resource_id, stored.id);
    assert_eq!(events[1].data["status"], "Settled");
}

#[tokio::test]
async fn payment_commands_reserve_once_claim_once_and_retain_deleted_keys() {
    use crate::domains::payment::{PaymentInstructions, SubmissionKey};
    use sea_orm::PaginatorTrait;
    use swissknife_types::SubmitPaymentRequest;

    let conn = connect().await;
    let wallet = seed_wallet(&conn, 100_000).await;
    let uow = SeaOrmPaymentUnitOfWork::new(conn.clone());
    let request = SubmitPaymentRequest {
        expected_offer_id: None,
        idempotency_key: Uuid::new_v4(),
        wallet_id: None,
        input: "synthetic invoice".into(),
        amount_msat: None,
        comment: None,
        maximum_fee_msat: Some(1_000),
    };
    let key = SubmissionKey::new(wallet, &request, None);
    let instructions = PaymentInstructions::Lightning {
        invoice: "synthetic invoice".into(),
        variable_amount_msat: None,
        maximum_fee_msat: 1_000,
    };
    let payment = pending_payment(wallet, 30_000, 0);
    let (a, b) = tokio::join!(
        uow.submit(&key, payment.clone(), 31_000, instructions.clone()),
        uow.submit(&key, payment, 31_000, instructions.clone())
    );
    let first = a.unwrap().payment;
    assert_eq!(first.id, b.unwrap().payment.id);
    assert_eq!(balance(&conn, wallet).await, (69_000, 31_000));

    let (a, b) = tokio::join!(uow.claim(), uow.claim());
    let claimed: Vec<_> = [a.unwrap(), b.unwrap()].into_iter().flatten().collect();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].payment.id, first.id);
    let events = SeaOrmClientEventRepository::new(conn.clone())
        .find_after(account_id(&conn, wallet).await, 0, 10)
        .await
        .unwrap();
    assert_eq!(
        events.iter().map(|e| e.event_type).collect::<Vec<_>>(),
        vec![ClientEventType::PaymentAccepted, ClientEventType::PaymentProcessing]
    );
    assert_eq!(
        super::models::account_notification::Entity::find()
            .count(&conn)
            .await
            .unwrap(),
        0
    );

    let after_restart = SeaOrmPaymentUnitOfWork::new(conn.clone());
    assert!(after_restart.claim().await.unwrap().is_none());
    assert!(after_restart.submitted(&key).await.unwrap().unwrap().dispatched);
    let changed = SubmissionKey::new(
        wallet,
        &SubmitPaymentRequest {
            comment: Some("different authorization".into()),
            ..request.clone()
        },
        None,
    );
    assert!(matches!(
        uow.submitted(&changed).await,
        Err(ApplicationError::Data(DataError::Conflict(_)))
    ));
    let rejected = SubmissionKey::new(
        wallet,
        &SubmitPaymentRequest {
            expected_offer_id: None,
            idempotency_key: Uuid::new_v4(),
            ..request
        },
        None,
    );
    assert!(uow
        .submit(&rejected, pending_payment(wallet, 90_000, 0), 90_000, instructions)
        .await
        .is_err());
    assert!(uow.submitted(&rejected).await.unwrap().is_none());

    uow.fail(Payment {
        status: PaymentStatus::Failed,
        ..first.clone()
    })
    .await
    .unwrap();
    assert_eq!(balance(&conn, wallet).await, (100_000, 0));
    SeaOrmPaymentRepository::new(conn.clone())
        .delete_many(crate::domains::payment::PaymentFilter {
            ids: Some(vec![first.id]),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(matches!(
        uow.submitted(&key).await,
        Err(ApplicationError::Data(DataError::Conflict(_)))
    ));
}

#[tokio::test]
async fn withdrawal_claims_fence_stale_workers_and_cancel_only_unstarted_owned_payments() {
    use super::models::withdrawal_command::{Column, Entity};
    use crate::domains::payment::{PaymentInstructions, SubmissionKey, WithdrawalProgress as Step};
    use sea_orm::sea_query::Expr;

    let conn = connect().await;
    let wallet = seed_wallet(&conn, 100_000_000).await;
    let other = seed_wallet(&conn, 100_000_000).await;
    let uow = SeaOrmPaymentUnitOfWork::new(conn.clone());
    let key = SubmissionKey {
        wallet_id: wallet,
        key: Uuid::new_v4(),
        fingerprint: "payment A".into(),
        agent_key_id: None,
    };
    let instructions = PaymentInstructions::Onchain {
        backend: "original".into(),
        address: "regtest destination".into(),
        maximum_fee_msat: 1_000_000,
    };
    let payment = Payment {
        wallet_id: wallet,
        ledger: Ledger::Onchain,
        amount_msat: 30_000_000,
        status: PaymentStatus::Pending,
        ..Default::default()
    };
    let (a, b) = tokio::join!(
        uow.submit(&key, payment.clone(), 31_000_000, instructions.clone()),
        uow.submit(&key, payment.clone(), 31_000_000, instructions.clone())
    );
    let id = a.unwrap().payment.id;
    assert_eq!(id, b.unwrap().payment.id);
    assert!(uow.cancel_withdrawal(Some(other), id).await.is_err());
    uow.cancel_withdrawal(Some(wallet), id).await.unwrap();
    uow.cancel_withdrawal(Some(wallet), id).await.unwrap();
    assert_eq!(balance(&conn, wallet).await, (100_000_000, 0));
    assert_eq!(
        uow.submitted(&key).await.unwrap().unwrap().response("").state,
        swissknife_types::PaymentExecutionState::Cancelled
    );
    let second = SubmissionKey {
        key: Uuid::new_v4(),
        ..key
    };
    let id = uow
        .submit(&second, payment, 31_000_000, instructions)
        .await
        .unwrap()
        .payment
        .id;
    assert!(uow.claim().await.unwrap().is_none());
    assert!(uow.claim_withdrawal("replacement").await.unwrap().is_none());
    let (a, b) = tokio::join!(uow.claim_withdrawal("original"), uow.claim_withdrawal("original"));
    let winners: Vec<_> = [a.unwrap(), b.unwrap()].into_iter().flatten().collect();
    assert_eq!(winners.len(), 1);
    let original = &winners[0];
    assert!(uow.cancel_withdrawal(Some(wallet), id).await.is_err());
    let funding = Step::Funding {
        owner: original.token,
        waiting: false,
    };
    Entity::update_many()
        .col_expr(
            Column::NextAttempt,
            Expr::value((Utc::now() - chrono::Duration::seconds(1)).naive_utc()),
        )
        .filter(Column::PaymentId.eq(id))
        .exec(&conn)
        .await
        .unwrap();
    let replacement = uow.claim_withdrawal("original").await.unwrap().unwrap();
    assert!(!uow.advance_withdrawal(original, Step::Expired, 0).await.unwrap());
    assert!(uow.advance_withdrawal(&replacement, funding, 0).await.unwrap());
    for waiting in [true, false, true] {
        let c = uow.claim_withdrawal("original").await.unwrap().unwrap();
        assert!(uow
            .advance_withdrawal(
                &c,
                Step::Funding {
                    owner: original.token,
                    waiting
                },
                0
            )
            .await
            .unwrap());
    }
    let events = SeaOrmClientEventRepository::new(conn.clone())
        .find_after(account_id(&conn, wallet).await, 0, 50)
        .await
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| e.resource_id == id && e.event_type == ClientEventType::WithdrawalWaiting)
            .count(),
        2
    );
    Entity::update_many()
        .col_expr(
            Column::ExpiresAt,
            Expr::value((Utc::now() - chrono::Duration::seconds(1)).naive_utc()),
        )
        .filter(Column::PaymentId.eq(id))
        .exec(&conn)
        .await
        .unwrap();
    let expired = uow.claim_withdrawal("replacement").await.unwrap().unwrap();
    assert!(!expired.original_backend);
    assert!(uow.advance_withdrawal(&expired, Step::Expired, 0).await.unwrap());
    assert!(!uow.advance_withdrawal(&expired, Step::Expired, 0).await.unwrap());
    assert_eq!(balance(&conn, wallet).await, (100_000_000, 0));
}

#[tokio::test]
async fn withdrawal_reconciliation_keeps_the_signed_payload_and_full_cap_until_confirmation() {
    use crate::domains::bitcoin::BtcBroadcastTransaction;
    use crate::domains::payment::{PaymentInstructions, Publication, SubmissionKey, WithdrawalProgress as Step};
    let conn = connect().await;
    let wallet = seed_wallet(&conn, 100_000_000).await;
    let uow = SeaOrmPaymentUnitOfWork::new(conn.clone());
    let key = SubmissionKey {
        wallet_id: wallet,
        key: Uuid::new_v4(),
        fingerprint: "approved capped payment".into(),
        agent_key_id: None,
    };
    let payment = Payment {
        wallet_id: wallet,
        ledger: Ledger::Onchain,
        amount_msat: 30_000_000,
        status: PaymentStatus::Pending,
        ..Default::default()
    };
    let original = uow
        .submit(
            &key,
            payment,
            35_000_000,
            PaymentInstructions::Onchain {
                backend: "node".into(),
                address: "external".into(),
                maximum_fee_msat: 5_000_000,
            },
        )
        .await
        .unwrap()
        .payment;
    let txid = "a4".repeat(32);
    let signed = BtcBroadcastTransaction::SignedPsbt {
        txid: txid.clone(),
        psbt: "signed fixture".into(),
    };
    let funding = uow.claim_withdrawal("node").await.unwrap().unwrap();
    assert!(uow
        .advance_withdrawal(
            &funding,
            Step::Funding {
                owner: funding.token,
                waiting: false
            },
            0
        )
        .await
        .unwrap());
    let preparing = uow.claim_withdrawal("node").await.unwrap().unwrap();
    let plan = crate::domains::bitcoin::BtcPreparedTransaction {
        txid: txid.clone(),
        fee_sat: 500,
        psbt: "funded fixture".into(),
        locked_utxos: vec![],
    };
    assert!(uow
        .advance_withdrawal(
            &preparing,
            Step::Planned {
                owner: funding.token,
                plan,
                waiting: false
            },
            0
        )
        .await
        .unwrap());
    let funding = uow.claim_withdrawal("node").await.unwrap().unwrap();
    assert!(uow
        .advance_withdrawal(
            &funding,
            Step::Signed {
                transaction: signed,
                fee_msat: 500_000,
                publication: Publication::Ready
            },
            0
        )
        .await
        .unwrap());
    for publication in [
        Publication::Unknown,
        Publication::Broadcast,
        Publication::Unknown,
        Publication::Broadcast,
    ] {
        let restarted = SeaOrmPaymentUnitOfWork::new(conn.clone());
        assert!(restarted.claim_withdrawal("other node").await.unwrap().is_none());
        let c = restarted.claim_withdrawal("node").await.unwrap().unwrap();
        let Step::Signed {
            transaction, fee_msat, ..
        } = c.progress.clone()
        else {
            panic!("signed artifact was lost");
        };
        assert_eq!(transaction.txid(), txid);
        assert_eq!(c.payment.bitcoin.as_ref().unwrap().txid, txid);
        assert_eq!(c.payment.fee_msat, Some(fee_msat));
        assert!(restarted
            .advance_withdrawal(
                &c,
                Step::Signed {
                    transaction,
                    fee_msat,
                    publication
                },
                0
            )
            .await
            .unwrap());
        assert_eq!(balance(&conn, wallet).await, (65_000_000, 35_000_000));
    }
    let mut settled = SeaOrmPaymentRepository::new(conn.clone())
        .find(original.id)
        .await
        .unwrap()
        .unwrap();
    settled.status = PaymentStatus::Settled;
    uow.settle(settled.clone()).await.unwrap();
    uow.settle(settled).await.unwrap();
    assert_eq!(balance(&conn, wallet).await, (69_500_000, 0));
    assert_eq!(
        uow.submitted(&key).await.unwrap().unwrap().response("").state,
        swissknife_types::PaymentExecutionState::Settled
    );
    assert!(uow.claim_withdrawal("node").await.unwrap().is_none());
}

#[tokio::test]
async fn offer_receipt_cursor_credit_history_and_event_commit_once_across_workers() {
    use super::offer_repository::SeaOrmOfferRepository;
    use crate::domains::offer::{CreateOfferRequest, NodeOffer, OfferRepository, OfferUpdate};
    let db = connect().await;
    let owner = seed_wallet(&db, 0).await;
    let repo = SeaOrmOfferRepository(db.clone());
    let (stored, _) = repo
        .register(
            owner,
            "cln:test",
            &CreateOfferRequest {
                wallet_id: None,
                idempotency_key: Uuid::new_v4(),
                amount_msat: None,
                description: "orders".into(),
                single_use: false,
            },
        )
        .await
        .unwrap();
    repo.activate(
        stored.offer.id,
        &NodeOffer {
            offer_id: "local".into(),
            bolt12: "lno1…".into(),
            active: true,
            label: None,
        },
    )
    .await
    .unwrap();
    let receipt = |index: u64, hash: &str, received: Option<u64>| OfferUpdate {
        updated_index: index,
        status: "paid".into(),
        local_offer_id: Some("local".into()),
        payment_hash: hash.into(),
        bolt12: Some("lni1…".into()),
        amount_msat: received,
        amount_received_msat: received,
        paid_at: Some(1795630000),
        invoice_created_at: Some(1795629999),
        invreq_payer_note: None,
    };
    // Failure on the second item rolls back the first credit, receipt and cursor.
    assert!(repo
        .project(
            "cln:test",
            0,
            vec![receipt(1, "hash-a", Some(123)), receipt(2, "hash-b", None)]
        )
        .await
        .is_err());
    assert_eq!(repo.cursor("cln:test").await.unwrap(), Some(0));
    assert_eq!(balance(&db, owner).await, (0, 0));
    let (a, b) = tokio::join!(
        repo.project("cln:test", 0, vec![receipt(1, "hash-a", Some(123))]),
        repo.project("cln:test", 0, vec![receipt(1, "hash-a", Some(123))])
    );
    a.unwrap();
    b.unwrap();
    assert_eq!(balance(&db, owner).await, (123, 0));
    assert_eq!(
        ClientEventEntity::find()
            .filter(client_event::Column::WalletId.eq(owner))
            .all(&db)
            .await
            .unwrap()
            .len(),
        1
    );
    let invoices = SeaOrmInvoiceRepository::new(db.clone());
    let invoice = invoices.find_by_payment_hash("hash-a").await.unwrap().unwrap();
    invoices
        .delete_many(crate::domains::invoice::InvoiceFilter {
            ids: Some(vec![invoice.id]),
            ..Default::default()
        })
        .await
        .unwrap();
    repo.project("cln:test", 1, vec![receipt(2, "hash-a", Some(123))])
        .await
        .unwrap();
    assert_eq!(balance(&db, owner).await, (123, 0));
    assert_eq!(repo.cursor("cln:test").await.unwrap(), Some(2));
}

#[tokio::test]
async fn spending_policy_serializes_two_wallets_and_retains_usage_across_payment_outcomes() {
    use super::spending_policy_repository::SeaOrmSpendingPolicyRepository;
    use crate::domains::agent::SpendingPolicyRepository;
    use swissknife_types::SpendingLimits;

    let conn = connect().await;
    let first_wallet = seed_wallet(&conn, 500_000).await;
    let account = account_id(&conn, first_wallet).await;
    let asset = SeaOrmAssetRepository::new(conn.clone())
        .find_native_btc_by_network(BtcNetwork::Regtest)
        .await
        .unwrap()
        .unwrap();
    let wallets = SeaOrmWalletRepository::new(conn.clone());
    let second_wallet = wallets.upsert(account, asset.id).await.unwrap().id;
    assert_ne!(first_wallet, second_wallet);
    wallets.credit(second_wallet, 500_000).await.unwrap();
    let policy = SeaOrmSpendingPolicyRepository(conn.clone());
    let limits = SpendingLimits {
        daily_msat: Some(100_000),
        ..Default::default()
    };
    policy.replace(account, limits.clone()).await.unwrap();

    let operations: Vec<_> = (0..8)
        .map(|index| {
            let worker = uow(&conn);
            let wallet = if index % 2 == 0 { first_wallet } else { second_wallet };
            async move { worker.reserve(pending_payment(wallet, 80_000, 0), 82_000).await }
        })
        .collect();
    let results = futures_util::future::join_all(operations).await;
    let mut admitted = vec![];
    for result in results {
        match result {
            Ok(payment) => admitted.push(payment),
            Err(ApplicationError::Data(DataError::Validation(reason))) => {
                assert!(reason.contains("daily spending limit"))
            }
            Err(other) => panic!("unexpected denial: {other}"),
        }
    }
    assert_eq!(admitted.len(), 1);
    let mut payment = admitted.pop().unwrap();
    let usage = policy.get(account).await.unwrap().unwrap().usage;
    assert_eq!(
        (usage.daily_msat, usage.weekly_msat, usage.monthly_msat),
        (82_000, 82_000, 82_000)
    );

    payment.status = PaymentStatus::Failed;
    uow(&conn).fail(payment.clone()).await.unwrap();
    uow(&conn).fail(payment.clone()).await.unwrap();
    assert_eq!(policy.get(account).await.unwrap().unwrap().usage.daily_msat, 0);
    assert_eq!(balance(&conn, payment.wallet_id).await, (500_000, 0));
    // A node may confirm payment after a premature failure. Book the truth even if
    // the operator has since lowered or disabled the policy.
    policy.disable(account).await.unwrap();
    payment.status = PaymentStatus::Settled;
    payment.fee_msat = Some(501);
    uow(&conn).settle(payment.clone()).await.unwrap();
    uow(&conn).settle(payment.clone()).await.unwrap();
    policy.replace(account, limits).await.unwrap();
    assert_eq!(policy.get(account).await.unwrap().unwrap().usage.daily_msat, 80_501);
    assert_eq!(balance(&conn, payment.wallet_id).await, (419_499, 0));
    super::models::payment::Entity::delete_by_id(payment.id)
        .exec(&conn)
        .await
        .unwrap();
    assert_eq!(policy.get(account).await.unwrap().unwrap().usage.monthly_msat, 80_501);
}
