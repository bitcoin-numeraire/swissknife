use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_1_wallet_table::Wallet, m20240420_2_ln_address_table::LnAddress};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        manager
            .alter_table(
                Table::alter()
                    .table(LnAddress::Table)
                    .add_column(uuid_null(LnAddress::AccountId))
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        execute(
            db,
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
        )
        .await?;

        let missing = db
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                format!(
                    "SELECT COUNT(*) AS count FROM {} WHERE {} IS NULL",
                    LnAddress::Table.to_string(),
                    LnAddress::AccountId.to_string(),
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

        execute(
            db,
            r#"
            ALTER TABLE ln_address
                DROP CONSTRAINT IF EXISTS fk_user,
                DROP CONSTRAINT IF EXISTS fk_ln_address_wallet
            "#
            .to_string(),
        )
        .await?;

        execute(
            db,
            r#"
            ALTER TABLE ln_address
                ALTER COLUMN account_id SET NOT NULL,
                ADD CONSTRAINT fk_ln_address_account
                    FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE,
                ADD CONSTRAINT fk_ln_address_wallet
                    FOREIGN KEY (account_id, wallet_id)
                    REFERENCES wallet(account_id, id) ON DELETE CASCADE
            "#
            .to_string(),
        )
        .await?;

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

async fn execute(db: &dyn ConnectionTrait, sql: String) -> Result<(), DbErr> {
    db.execute(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .map(|_| ())
}
