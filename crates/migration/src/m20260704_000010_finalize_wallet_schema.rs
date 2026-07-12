use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        manager
            .get_connection()
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                r#"
                ALTER TABLE wallet
                    ALTER COLUMN account_id SET NOT NULL,
                    ALTER COLUMN asset_id SET NOT NULL,
                    ADD CONSTRAINT fk_wallet_account
                        FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE,
                    ADD CONSTRAINT fk_wallet_asset
                        FOREIGN KEY (asset_id) REFERENCES asset(id) ON DELETE RESTRICT,
                    ADD CONSTRAINT chk_wallet_reserved_amount
                        CHECK (reserved_amount >= 0),
                    DROP COLUMN IF EXISTS user_id
                "#
                .to_string(),
            ))
            .await
            .map(|_| ())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "the account-owned wallet cutover is irreversible".to_string(),
        ))
    }
}
