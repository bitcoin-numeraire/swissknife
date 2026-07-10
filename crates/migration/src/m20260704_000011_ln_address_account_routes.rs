use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::{prelude::*, schema::*};

use crate::{
    m20240420_1_wallet_table::Wallet, m20240420_2_ln_address_table::LnAddress, m20260704_000001_account_table::Account,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LnAddress::Table)
                    .add_column(uuid_null(LnAddress::AccountId))
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        let backend = db.get_database_backend();
        db.execute(Statement::from_string(
            backend,
            format!(
                r#"
                UPDATE {ln_address}
                SET {account_id} = (
                    SELECT {wallet}.{wallet_account_id}
                    FROM {wallet}
                    WHERE {wallet}.{wallet_id} = {ln_address}.{ln_address_wallet_id}
                )
                WHERE {account_id} IS NULL
                "#,
                ln_address = LnAddress::Table.to_string(),
                account_id = LnAddress::AccountId.to_string(),
                wallet = Wallet::Table.to_string(),
                wallet_account_id = Wallet::AccountId.to_string(),
                wallet_id = Wallet::Id.to_string(),
                ln_address_wallet_id = LnAddress::WalletId.to_string(),
            ),
        ))
        .await?;

        let missing = db
            .query_one(Statement::from_string(
                backend,
                format!(
                    r#"
                    SELECT COUNT(*) AS count
                    FROM {ln_address}
                    WHERE {account_id} IS NULL
                    "#,
                    ln_address = LnAddress::Table.to_string(),
                    account_id = LnAddress::AccountId.to_string(),
                ),
            ))
            .await?
            .ok_or_else(|| DbErr::Migration("ln_address account backfill count returned no row".to_string()))?
            .try_get::<i64>("", "count")?;
        if missing > 0 {
            return Err(DbErr::Migration(format!(
                "{missing} lightning address rows could not be mapped to an account"
            )));
        }

        match backend {
            DatabaseBackend::Postgres => {
                execute(
                    db,
                    backend,
                    r#"
                    ALTER TABLE ln_address
                        ALTER COLUMN account_id SET NOT NULL,
                        ADD CONSTRAINT fk_ln_address_account
                            FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE
                    "#,
                )
                .await?;
            }
            DatabaseBackend::Sqlite => rebuild_sqlite_ln_address(manager).await?,
            DatabaseBackend::MySql => {}
        }

        manager
            .create_index(
                Index::create()
                    .name("idx_ln_address_account")
                    .table(LnAddress::Table)
                    .col(LnAddress::AccountId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "the lightning address account cutover is irreversible".to_string(),
        ))
    }
}

async fn rebuild_sqlite_ln_address(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    let backup = Alias::new("ln_address_backup");

    execute(db, DatabaseBackend::Sqlite, "PRAGMA foreign_keys = OFF").await?;
    execute(
        db,
        DatabaseBackend::Sqlite,
        r#"
        CREATE TABLE ln_address_backup AS
        SELECT id, wallet_id, username, active, created_at, updated_at, allows_nostr, nostr_pubkey, account_id
        FROM ln_address
        "#,
    )
    .await?;
    manager
        .drop_table(Table::drop().table(LnAddress::Table).to_owned())
        .await?;
    manager
        .create_table(
            Table::create()
                .table(LnAddress::Table)
                .col(uuid(LnAddress::Id).primary_key())
                .col(uuid_uniq(LnAddress::WalletId))
                .col(string_len_uniq(LnAddress::Username, 255))
                .col(boolean(LnAddress::Active).default(true))
                .col(timestamp(LnAddress::CreatedAt).default(Expr::current_timestamp()))
                .col(timestamp_null(LnAddress::UpdatedAt))
                .col(boolean(LnAddress::AllowsNostr).default(false))
                .col(string_len_null(LnAddress::NostrPubkey, 255))
                .col(uuid(LnAddress::AccountId))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_ln_address_wallet")
                        .from(LnAddress::Table, LnAddress::WalletId)
                        .to(Wallet::Table, Wallet::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_ln_address_account")
                        .from(LnAddress::Table, LnAddress::AccountId)
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
        INSERT INTO ln_address (
            id, wallet_id, username, active, created_at, updated_at, allows_nostr, nostr_pubkey, account_id
        )
        SELECT id, wallet_id, username, active, created_at, updated_at, allows_nostr, nostr_pubkey, account_id
        FROM ln_address_backup
        "#,
    )
    .await?;
    manager.drop_table(Table::drop().table(backup).to_owned()).await?;
    execute(db, DatabaseBackend::Sqlite, "PRAGMA foreign_keys = ON").await
}

async fn execute(db: &dyn ConnectionTrait, backend: DatabaseBackend, sql: &str) -> Result<(), DbErr> {
    db.execute(Statement::from_string(backend, sql.to_string()))
        .await
        .map(|_| ())
}
