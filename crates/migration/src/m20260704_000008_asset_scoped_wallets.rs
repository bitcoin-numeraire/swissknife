use std::collections::BTreeSet;

use sea_orm::{ConnectionTrait, DatabaseBackend, QueryResult, Statement};
use sea_orm_migration::{prelude::*, schema::*};
use uuid::Uuid;

use crate::{
    m20240420_1_wallet_table::Wallet, m20240420_2_ln_address_table::LnAddress, m20240420_3_invoice_table::Invoice,
    m20240420_4_payment_table::Payment, m20251224_162538_btc_address_table::BtcAddress,
    m20260609_143600_wallet_balance_table::WalletBalance, m20260704_000002_auth_identity_table::AuthIdentity,
    m20260704_000005_asset_table::Asset,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

const LEGACY_AUTH_PROVIDER: &str = "oauth2";
const ACTIVE_NETWORK_ENV: &str = "SWISSKNIFE_IDENTITY_MIGRATION_ACTIVE_BITCOIN_NETWORK";
const TESTNET_NETWORK_ENV: &str = "SWISSKNIFE_IDENTITY_MIGRATION_TESTNET_NETWORK";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_wallet_column(manager, uuid_null(Wallet::AccountId)).await?;
        add_wallet_column(manager, uuid_null(Wallet::AssetId)).await?;
        add_wallet_column(manager, text_null(Wallet::Label)).await?;
        let mut available_amount = big_integer(Wallet::AvailableAmount);
        available_amount.default(0);
        add_wallet_column(manager, available_amount).await?;
        let mut reserved_amount = big_integer(Wallet::ReservedAmount);
        reserved_amount.default(0);
        add_wallet_column(manager, reserved_amount).await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wallet_account_asset")
                    .table(Wallet::Table)
                    .col(Wallet::AccountId)
                    .col(Wallet::AssetId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WalletAssetMigration::Table)
                    .if_not_exists()
                    .col(uuid(WalletAssetMigration::OldWalletId))
                    .col(string_len(WalletAssetMigration::OldCurrency, 255))
                    .col(uuid(WalletAssetMigration::NewWalletId))
                    .primary_key(
                        Index::create()
                            .name("pk_wallet_asset_migration")
                            .col(WalletAssetMigration::OldWalletId)
                            .col(WalletAssetMigration::OldCurrency),
                    )
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        backfill_asset_wallets(db).await?;
        repoint_resources(db).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WalletAssetMigration::Table).to_owned())
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_wallet_account_asset")
                    .table(Wallet::Table)
                    .to_owned(),
            )
            .await?;
        drop_wallet_column(manager, Wallet::ReservedAmount).await?;
        drop_wallet_column(manager, Wallet::AvailableAmount).await?;
        drop_wallet_column(manager, Wallet::Label).await?;
        drop_wallet_column(manager, Wallet::AssetId).await?;
        drop_wallet_column(manager, Wallet::AccountId).await?;

        Ok(())
    }
}

async fn add_wallet_column(manager: &SchemaManager<'_>, column: ColumnDef) -> Result<(), DbErr> {
    manager
        .alter_table(Table::alter().table(Wallet::Table).add_column(column).to_owned())
        .await
}

async fn drop_wallet_column(manager: &SchemaManager<'_>, column: Wallet) -> Result<(), DbErr> {
    manager
        .alter_table(Table::alter().table(Wallet::Table).drop_column(column).to_owned())
        .await
}

async fn backfill_asset_wallets(db: &dyn ConnectionTrait) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let wallets = query_all(
        db,
        backend,
        format!(
            r#"
            SELECT {id_expr} AS id, {user_id}
            FROM {wallet}
            ORDER BY {user_id}
            "#,
            id_expr = uuid_select_expr(backend, &Wallet::Id.to_string()),
            user_id = Wallet::UserId.to_string(),
            wallet = Wallet::Table.to_string(),
        ),
    )
    .await?;

    for wallet in wallets {
        let old_wallet_id = row_uuid(&wallet, "id")?;
        let user_id = wallet.try_get::<String>("", "user_id")?;
        let account_id = account_id_for_subject(db, backend, &user_id).await?;
        let mut currencies = currencies_for_wallet(db, backend, old_wallet_id).await?;

        if currencies.is_empty() || wallet_has_identity_level_routes(db, backend, old_wallet_id).await? {
            currencies.insert(active_legacy_currency()?);
        }

        for (index, currency) in currencies.iter().enumerate() {
            let asset_id = asset_id_for_legacy_currency(db, backend, currency).await?;
            let (available_amount, reserved_amount) =
                balance_for_wallet_currency(db, backend, old_wallet_id, currency).await?;
            let new_wallet_id = if index == 0 { old_wallet_id } else { Uuid::new_v4() };

            if index == 0 {
                update_existing_wallet(
                    db,
                    backend,
                    old_wallet_id,
                    account_id,
                    asset_id,
                    available_amount,
                    reserved_amount,
                )
                .await?;
            } else {
                insert_asset_wallet(
                    db,
                    backend,
                    new_wallet_id,
                    account_id,
                    asset_id,
                    available_amount,
                    reserved_amount,
                )
                .await?;
            }

            insert_mapping(db, backend, old_wallet_id, currency, new_wallet_id).await?;
        }
    }

    Ok(())
}

async fn repoint_resources(db: &dyn ConnectionTrait) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    execute(
        db,
        backend,
        format!(
            r#"
            UPDATE {payment}
            SET {wallet_id} = (
                SELECT {new_wallet_id}
                FROM {mapping}
                WHERE {old_wallet_id} = {payment}.{wallet_id}
                  AND {old_currency} = {payment}.{currency}
            )
            WHERE EXISTS (
                SELECT 1
                FROM {mapping}
                WHERE {old_wallet_id} = {payment}.{wallet_id}
                  AND {old_currency} = {payment}.{currency}
            )
            "#,
            payment = Payment::Table.to_string(),
            wallet_id = Payment::WalletId.to_string(),
            currency = Payment::Currency.to_string(),
            mapping = WalletAssetMigration::Table.to_string(),
            old_wallet_id = WalletAssetMigration::OldWalletId.to_string(),
            old_currency = WalletAssetMigration::OldCurrency.to_string(),
            new_wallet_id = WalletAssetMigration::NewWalletId.to_string(),
        ),
    )
    .await?;

    execute(
        db,
        backend,
        format!(
            r#"
            UPDATE {invoice}
            SET {wallet_id} = (
                SELECT {new_wallet_id}
                FROM {mapping}
                WHERE {old_wallet_id} = {invoice}.{wallet_id}
                  AND {old_currency} = {invoice}.{currency}
            )
            WHERE EXISTS (
                SELECT 1
                FROM {mapping}
                WHERE {old_wallet_id} = {invoice}.{wallet_id}
                  AND {old_currency} = {invoice}.{currency}
            )
            "#,
            invoice = Invoice::Table.to_string(),
            wallet_id = Invoice::WalletId.to_string(),
            currency = Invoice::Currency.to_string(),
            mapping = WalletAssetMigration::Table.to_string(),
            old_wallet_id = WalletAssetMigration::OldWalletId.to_string(),
            old_currency = WalletAssetMigration::OldCurrency.to_string(),
            new_wallet_id = WalletAssetMigration::NewWalletId.to_string(),
        ),
    )
    .await?;

    if !has_identity_level_routes(db, backend).await? {
        return Ok(());
    }

    let active_currency = sql_literal(&active_legacy_currency()?);
    for table in [BtcAddress::Table.to_string(), LnAddress::Table.to_string()] {
        execute(
            db,
            backend,
            format!(
                r#"
                UPDATE {table}
                SET {wallet_id} = (
                    SELECT {new_wallet_id}
                    FROM {mapping}
                    WHERE {old_wallet_id} = {table}.{wallet_id}
                      AND {old_currency} = '{active_currency}'
                )
                WHERE EXISTS (
                    SELECT 1
                    FROM {mapping}
                    WHERE {old_wallet_id} = {table}.{wallet_id}
                      AND {old_currency} = '{active_currency}'
                )
                "#,
                table = table,
                wallet_id = BtcAddress::WalletId.to_string(),
                mapping = WalletAssetMigration::Table.to_string(),
                old_wallet_id = WalletAssetMigration::OldWalletId.to_string(),
                old_currency = WalletAssetMigration::OldCurrency.to_string(),
                new_wallet_id = WalletAssetMigration::NewWalletId.to_string(),
            ),
        )
        .await?;
    }

    Ok(())
}

async fn account_id_for_subject(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    subject: &str,
) -> Result<Uuid, DbErr> {
    let row = query_one(
        db,
        backend,
        format!(
            r#"
            SELECT {account_id_expr} AS account_id
            FROM {auth_identity}
            WHERE {provider_col} = '{provider}'
              AND {subject_col} = '{subject}'
            "#,
            account_id_expr = uuid_select_expr(backend, &AuthIdentity::AccountId.to_string()),
            auth_identity = AuthIdentity::Table.to_string(),
            provider_col = AuthIdentity::Provider.to_string(),
            subject_col = AuthIdentity::Subject.to_string(),
            provider = sql_literal(LEGACY_AUTH_PROVIDER),
            subject = sql_literal(subject),
        ),
    )
    .await?
    .ok_or_else(|| {
        DbErr::Migration(format!(
            "missing account identity for migrated wallet subject {subject}"
        ))
    })?;

    row_uuid(&row, "account_id")
}

async fn currencies_for_wallet(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    old_wallet_id: Uuid,
) -> Result<BTreeSet<String>, DbErr> {
    let old_wallet_id = uuid_literal(backend, old_wallet_id);
    let rows = query_all(
        db,
        backend,
        format!(
            r#"
            SELECT {currency}
            FROM (
                SELECT {currency} FROM {wallet_balance} WHERE {wallet_id} = {old_wallet_id}
                UNION
                SELECT {currency} FROM {payment} WHERE {wallet_id} = {old_wallet_id}
                UNION
                SELECT {currency} FROM {invoice} WHERE {wallet_id} = {old_wallet_id}
            ) wallet_currencies
            ORDER BY {currency}
            "#,
            currency = WalletBalance::Currency.to_string(),
            wallet_balance = WalletBalance::Table.to_string(),
            payment = Payment::Table.to_string(),
            invoice = Invoice::Table.to_string(),
            wallet_id = WalletBalance::WalletId.to_string(),
            old_wallet_id = old_wallet_id,
        ),
    )
    .await?;

    rows.into_iter()
        .map(|row| row.try_get::<String>("", "currency"))
        .collect()
}

async fn wallet_has_identity_level_routes(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    old_wallet_id: Uuid,
) -> Result<bool, DbErr> {
    let old_wallet_id = uuid_literal(backend, old_wallet_id);
    let row = query_one(
        db,
        backend,
        format!(
            r#"
            SELECT COUNT(*) AS count
            FROM (
                SELECT {wallet_id} FROM {btc_address} WHERE {wallet_id} = {old_wallet_id}
                UNION ALL
                SELECT {wallet_id} FROM {ln_address} WHERE {wallet_id} = {old_wallet_id}
            ) wallet_routes
            "#,
            wallet_id = BtcAddress::WalletId.to_string(),
            btc_address = BtcAddress::Table.to_string(),
            ln_address = LnAddress::Table.to_string(),
            old_wallet_id = old_wallet_id,
        ),
    )
    .await?
    .ok_or_else(|| DbErr::Migration("route count query returned no row".to_string()))?;

    Ok(row.try_get::<i64>("", "count")? > 0)
}

async fn has_identity_level_routes(db: &dyn ConnectionTrait, backend: DatabaseBackend) -> Result<bool, DbErr> {
    let row = query_one(
        db,
        backend,
        format!(
            r#"
            SELECT COUNT(*) AS count
            FROM (
                SELECT {wallet_id} FROM {btc_address}
                UNION ALL
                SELECT {wallet_id} FROM {ln_address}
            ) wallet_routes
            "#,
            wallet_id = BtcAddress::WalletId.to_string(),
            btc_address = BtcAddress::Table.to_string(),
            ln_address = LnAddress::Table.to_string(),
        ),
    )
    .await?
    .ok_or_else(|| DbErr::Migration("route count query returned no row".to_string()))?;

    Ok(row.try_get::<i64>("", "count")? > 0)
}

async fn balance_for_wallet_currency(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    old_wallet_id: Uuid,
    currency: &str,
) -> Result<(i64, i64), DbErr> {
    let row = query_one(
        db,
        backend,
        format!(
            r#"
            SELECT {available_amount}, {reserved_amount}
            FROM {wallet_balance}
            WHERE {wallet_id} = {old_wallet_id}
              AND {currency_col} = '{currency}'
            "#,
            available_amount = WalletBalance::AvailableAmount.to_string(),
            reserved_amount = WalletBalance::ReservedAmount.to_string(),
            wallet_balance = WalletBalance::Table.to_string(),
            wallet_id = WalletBalance::WalletId.to_string(),
            old_wallet_id = uuid_literal(backend, old_wallet_id),
            currency_col = WalletBalance::Currency.to_string(),
            currency = sql_literal(currency),
        ),
    )
    .await?;

    match row {
        Some(row) => Ok((
            row.try_get::<i64>("", "available_amount")?,
            row.try_get::<i64>("", "reserved_amount")?,
        )),
        None => Ok((0, 0)),
    }
}

async fn asset_id_for_legacy_currency(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    currency: &str,
) -> Result<Uuid, DbErr> {
    let network = network_for_legacy_currency(currency)?;
    let row = query_one(
        db,
        backend,
        format!(
            r#"
            SELECT {id_expr} AS id
            FROM {asset}
            WHERE {protocol} = 'bitcoin'
              AND {network_col} = '{network}'
              AND {asset_ref} = 'native'
            "#,
            id_expr = uuid_select_expr(backend, &Asset::Id.to_string()),
            asset = Asset::Table.to_string(),
            protocol = Asset::Protocol.to_string(),
            network_col = Asset::Network.to_string(),
            asset_ref = Asset::AssetRef.to_string(),
            network = sql_literal(&network),
        ),
    )
    .await?
    .ok_or_else(|| DbErr::Migration(format!("missing native BTC asset for legacy currency {currency}")))?;

    row_uuid(&row, "id")
}

async fn update_existing_wallet(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    wallet_id: Uuid,
    account_id: Uuid,
    asset_id: Uuid,
    available_amount: i64,
    reserved_amount: i64,
) -> Result<(), DbErr> {
    execute(
        db,
        backend,
        format!(
            r#"
            UPDATE {wallet}
            SET {account_id_col} = {account_id},
                {asset_id_col} = {asset_id},
                {available_amount_col} = {available_amount},
                {reserved_amount_col} = {reserved_amount},
                {updated_at} = CURRENT_TIMESTAMP
            WHERE {id} = {wallet_id}
            "#,
            wallet = Wallet::Table.to_string(),
            account_id_col = Wallet::AccountId.to_string(),
            account_id = uuid_literal(backend, account_id),
            asset_id_col = Wallet::AssetId.to_string(),
            asset_id = uuid_literal(backend, asset_id),
            available_amount_col = Wallet::AvailableAmount.to_string(),
            reserved_amount_col = Wallet::ReservedAmount.to_string(),
            updated_at = Wallet::UpdatedAt.to_string(),
            id = Wallet::Id.to_string(),
            wallet_id = uuid_literal(backend, wallet_id),
        ),
    )
    .await
}

async fn insert_asset_wallet(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    wallet_id: Uuid,
    account_id: Uuid,
    asset_id: Uuid,
    available_amount: i64,
    reserved_amount: i64,
) -> Result<(), DbErr> {
    execute(
        db,
        backend,
        format!(
            r#"
            INSERT INTO {wallet} (
                {id},
                {user_id},
                {account_id_col},
                {asset_id_col},
                {available_amount_col},
                {reserved_amount_col},
                {created_at}
            )
            VALUES (
                {wallet_id},
                '{legacy_user_id}',
                {account_id},
                {asset_id},
                {available_amount},
                {reserved_amount},
                CURRENT_TIMESTAMP
            )
            "#,
            wallet = Wallet::Table.to_string(),
            id = Wallet::Id.to_string(),
            wallet_id = uuid_literal(backend, wallet_id),
            user_id = Wallet::UserId.to_string(),
            legacy_user_id = sql_literal(&legacy_user_id(account_id, asset_id)),
            account_id_col = Wallet::AccountId.to_string(),
            account_id = uuid_literal(backend, account_id),
            asset_id_col = Wallet::AssetId.to_string(),
            asset_id = uuid_literal(backend, asset_id),
            available_amount_col = Wallet::AvailableAmount.to_string(),
            reserved_amount_col = Wallet::ReservedAmount.to_string(),
            created_at = Wallet::CreatedAt.to_string(),
        ),
    )
    .await
}

async fn insert_mapping(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    old_wallet_id: Uuid,
    old_currency: &str,
    new_wallet_id: Uuid,
) -> Result<(), DbErr> {
    execute(
        db,
        backend,
        format!(
            r#"
            INSERT INTO {mapping} ({old_wallet_id_col}, {old_currency_col}, {new_wallet_id_col})
            VALUES ({old_wallet_id}, '{old_currency}', {new_wallet_id})
            ON CONFLICT({old_wallet_id_col}, {old_currency_col}) DO NOTHING
            "#,
            mapping = WalletAssetMigration::Table.to_string(),
            old_wallet_id_col = WalletAssetMigration::OldWalletId.to_string(),
            old_currency_col = WalletAssetMigration::OldCurrency.to_string(),
            new_wallet_id_col = WalletAssetMigration::NewWalletId.to_string(),
            old_wallet_id = uuid_literal(backend, old_wallet_id),
            old_currency = sql_literal(old_currency),
            new_wallet_id = uuid_literal(backend, new_wallet_id),
        ),
    )
    .await
}

fn network_for_legacy_currency(currency: &str) -> Result<String, DbErr> {
    match currency {
        "Bitcoin" => Ok("bitcoin/mainnet".to_string()),
        "Regtest" => Ok("bitcoin/regtest".to_string()),
        "Simnet" => Ok("bitcoin/simnet".to_string()),
        "Signet" => Ok("bitcoin/signet".to_string()),
        "BitcoinTestnet" => testnet_network(),
        other => Err(DbErr::Migration(format!(
            "cannot map unsupported legacy wallet currency {other} to an asset"
        ))),
    }
}

fn testnet_network() -> Result<String, DbErr> {
    if let Ok(value) = std::env::var(TESTNET_NETWORK_ENV) {
        return parse_active_network(&value).and_then(|network| match network.as_str() {
            "bitcoin/testnet" | "bitcoin/testnet4" => Ok(network),
            _ => Err(DbErr::Migration(format!(
                "{TESTNET_NETWORK_ENV} must be testnet or testnet4 when BitcoinTestnet rows exist"
            ))),
        });
    }

    let active = active_network()?;
    match active.as_str() {
        "bitcoin/testnet" | "bitcoin/testnet4" => Ok(active),
        _ => Err(DbErr::Migration(format!(
            "BitcoinTestnet rows require {TESTNET_NETWORK_ENV}=testnet|testnet4 or an active testnet network"
        ))),
    }
}

fn active_legacy_currency() -> Result<String, DbErr> {
    match active_network()?.as_str() {
        "bitcoin/mainnet" => Ok("Bitcoin".to_string()),
        "bitcoin/testnet" | "bitcoin/testnet4" => Ok("BitcoinTestnet".to_string()),
        "bitcoin/regtest" => Ok("Regtest".to_string()),
        "bitcoin/simnet" => Ok("Simnet".to_string()),
        "bitcoin/signet" => Ok("Signet".to_string()),
        network => Err(DbErr::Migration(format!(
            "unsupported active Bitcoin network {network}"
        ))),
    }
}

fn active_network() -> Result<String, DbErr> {
    let value = std::env::var(ACTIVE_NETWORK_ENV).map_err(|_| {
        DbErr::Migration(format!(
            "{ACTIVE_NETWORK_ENV} is required to route empty wallets, Bitcoin addresses, and Lightning addresses"
        ))
    })?;

    parse_active_network(&value)
}

fn parse_active_network(value: &str) -> Result<String, DbErr> {
    match value.trim().to_ascii_lowercase().as_str() {
        "bitcoin" | "mainnet" | "bitcoin/mainnet" => Ok("bitcoin/mainnet".to_string()),
        "testnet" | "testnet3" | "bitcoin/testnet" => Ok("bitcoin/testnet".to_string()),
        "testnet4" | "bitcoin/testnet4" => Ok("bitcoin/testnet4".to_string()),
        "regtest" | "bitcoin/regtest" => Ok("bitcoin/regtest".to_string()),
        "simnet" | "bitcoin/simnet" => Ok("bitcoin/simnet".to_string()),
        "signet" | "bitcoin/signet" => Ok("bitcoin/signet".to_string()),
        other => Err(DbErr::Migration(format!("unsupported Bitcoin network {other}"))),
    }
}

fn legacy_user_id(account_id: Uuid, asset_id: Uuid) -> String {
    format!("{account_id}:{asset_id}")
}

async fn query_all(db: &dyn ConnectionTrait, backend: DatabaseBackend, sql: String) -> Result<Vec<QueryResult>, DbErr> {
    db.query_all(Statement::from_string(backend, sql)).await
}

async fn query_one(
    db: &dyn ConnectionTrait,
    backend: DatabaseBackend,
    sql: String,
) -> Result<Option<QueryResult>, DbErr> {
    db.query_one(Statement::from_string(backend, sql)).await
}

async fn execute(db: &dyn ConnectionTrait, backend: DatabaseBackend, sql: String) -> Result<(), DbErr> {
    db.execute(Statement::from_string(backend, sql)).await?;
    Ok(())
}

fn uuid_literal(backend: DatabaseBackend, value: Uuid) -> String {
    match backend {
        DatabaseBackend::Sqlite => format!("X'{}'", value.simple()),
        _ => format!("'{value}'"),
    }
}

fn uuid_select_expr(backend: DatabaseBackend, column: &str) -> String {
    match backend {
        DatabaseBackend::Sqlite => format!("lower(hex({column}))"),
        DatabaseBackend::Postgres => format!("{column}::text"),
        _ => column.to_string(),
    }
}

fn row_uuid(row: &QueryResult, column: &str) -> Result<Uuid, DbErr> {
    let value = row.try_get::<String>("", column)?;
    Uuid::parse_str(&value).map_err(|err| DbErr::Migration(format!("invalid UUID in {column}: {err}")))
}

fn sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

#[derive(DeriveIden)]
pub(crate) enum WalletAssetMigration {
    Table,
    OldWalletId,
    OldCurrency,
    NewWalletId,
}
