use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::{prelude::*, schema::*};

use crate::{
    m20240420_1_wallet_table::Wallet, m20241009_6_api_key_table::ApiKey, m20260704_000001_account_table::Account,
    m20260704_000005_asset_table::Asset,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        match db.get_database_backend() {
            DatabaseBackend::Postgres => {
                execute(
                    db,
                    DatabaseBackend::Postgres,
                    "ALTER TABLE api_key DROP CONSTRAINT IF EXISTS api_key_user_id_fkey",
                )
                .await?;
                execute(
                    db,
                    DatabaseBackend::Postgres,
                    "ALTER TABLE api_key DROP COLUMN IF EXISTS user_id",
                )
                .await?;
                execute(
                    db,
                    DatabaseBackend::Postgres,
                    r#"
                    ALTER TABLE api_key
                        ADD CONSTRAINT fk_api_key_account
                        FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE
                    "#,
                )
                .await?;
                execute(
                    db,
                    DatabaseBackend::Postgres,
                    r#"
                    ALTER TABLE wallet
                        ALTER COLUMN account_id SET NOT NULL,
                        ALTER COLUMN asset_id SET NOT NULL,
                        ADD CONSTRAINT fk_wallet_account
                            FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE,
                        ADD CONSTRAINT fk_wallet_asset
                            FOREIGN KEY (asset_id) REFERENCES asset(id) ON DELETE RESTRICT,
                        ADD CONSTRAINT chk_wallet_available_amount
                            CHECK (available_amount >= 0),
                        ADD CONSTRAINT chk_wallet_reserved_amount
                            CHECK (reserved_amount >= 0),
                        DROP COLUMN IF EXISTS user_id
                    "#,
                )
                .await
            }
            DatabaseBackend::Sqlite => rebuild_sqlite_owned_tables(manager).await,
            DatabaseBackend::MySql => Ok(()),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn rebuild_sqlite_owned_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    execute(db, DatabaseBackend::Sqlite, "PRAGMA foreign_keys = OFF").await?;
    rebuild_sqlite_api_key(manager).await?;
    rebuild_sqlite_wallet(manager).await?;
    execute(db, DatabaseBackend::Sqlite, "PRAGMA foreign_keys = ON").await
}

async fn rebuild_sqlite_api_key(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    let legacy = Alias::new("api_key_legacy");

    execute(
        db,
        DatabaseBackend::Sqlite,
        "DROP INDEX IF EXISTS idx_api_key_account_id",
    )
    .await?;
    manager
        .rename_table(Table::rename().table(ApiKey::Table, legacy.clone()).to_owned())
        .await?;
    manager
        .create_table(
            Table::create()
                .table(ApiKey::Table)
                .col(uuid(ApiKey::Id).primary_key())
                .col(string_len(ApiKey::Name, 255))
                .col(binary_len_uniq(ApiKey::KeyHash, 32))
                .col(json(ApiKey::Permissions))
                .col(text_null(ApiKey::Description))
                .col(timestamp(ApiKey::CreatedAt).default(Expr::current_timestamp()))
                .col(timestamp_null(ApiKey::ExpiresAt))
                .col(uuid(ApiKey::AccountId))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_api_key_account")
                        .from(ApiKey::Table, ApiKey::AccountId)
                        .to(Account::Table, Account::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    execute(
        db,
        DatabaseBackend::Sqlite,
        r#"
        INSERT INTO api_key (
            id, name, key_hash, permissions, description, created_at, expires_at, account_id
        )
        SELECT id, name, key_hash, permissions, description, created_at, expires_at, account_id
        FROM api_key_legacy
        "#,
    )
    .await?;
    manager.drop_table(Table::drop().table(legacy).to_owned()).await?;
    manager
        .create_index(
            Index::create()
                .name("idx_api_key_account_id")
                .table(ApiKey::Table)
                .col(ApiKey::AccountId)
                .to_owned(),
        )
        .await
}

async fn rebuild_sqlite_wallet(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    let backup = Alias::new("wallet_backup");

    execute(
        db,
        DatabaseBackend::Sqlite,
        "DROP INDEX IF EXISTS idx_wallet_account_asset",
    )
    .await?;
    execute(
        db,
        DatabaseBackend::Sqlite,
        r#"
        CREATE TABLE wallet_backup AS
        SELECT id, account_id, asset_id, label, available_amount, reserved_amount, created_at, updated_at
        FROM wallet
        "#,
    )
    .await?;
    manager
        .drop_table(Table::drop().table(Wallet::Table).to_owned())
        .await?;
    manager
        .create_table(
            Table::create()
                .table(Wallet::Table)
                .col(uuid(Wallet::Id).primary_key())
                .col(uuid(Wallet::AccountId))
                .col(uuid(Wallet::AssetId))
                .col(text_null(Wallet::Label))
                .col(big_integer(Wallet::AvailableAmount).default(0))
                .col(big_integer(Wallet::ReservedAmount).default(0))
                .col(timestamp(Wallet::CreatedAt).default(Expr::current_timestamp()))
                .col(timestamp_null(Wallet::UpdatedAt))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_wallet_account")
                        .from(Wallet::Table, Wallet::AccountId)
                        .to(Account::Table, Account::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_wallet_asset")
                        .from(Wallet::Table, Wallet::AssetId)
                        .to(Asset::Table, Asset::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .check(Expr::col(Wallet::AvailableAmount).gte(0))
                .check(Expr::col(Wallet::ReservedAmount).gte(0))
                .to_owned(),
        )
        .await?;

    execute(
        db,
        DatabaseBackend::Sqlite,
        r#"
        INSERT INTO wallet (
            id, account_id, asset_id, label, available_amount, reserved_amount, created_at, updated_at
        )
        SELECT id, account_id, asset_id, label, available_amount, reserved_amount, created_at, updated_at
        FROM wallet_backup
        "#,
    )
    .await?;
    manager.drop_table(Table::drop().table(backup).to_owned()).await?;
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
    Ok(())
}

async fn execute(db: &dyn ConnectionTrait, backend: DatabaseBackend, sql: &str) -> Result<(), DbErr> {
    db.execute(Statement::from_string(backend, sql.to_string()))
        .await
        .map(|_| ())
}
