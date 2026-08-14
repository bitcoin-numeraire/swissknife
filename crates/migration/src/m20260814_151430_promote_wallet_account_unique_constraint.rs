use sea_orm::{ConnectionTrait, DatabaseBackend};
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
            .execute_unprepared(
                r#"
                ALTER TABLE ln_address
                    DROP CONSTRAINT fk_ln_address_wallet;

                ALTER TABLE wallet
                    ADD CONSTRAINT uq_wallet_account_id
                    UNIQUE USING INDEX idx_wallet_account_id;

                ALTER TABLE ln_address
                    ADD CONSTRAINT fk_ln_address_wallet
                    FOREIGN KEY (account_id, wallet_id)
                    REFERENCES wallet(account_id, id)
                    ON DELETE CASCADE;
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE ln_address
                    DROP CONSTRAINT fk_ln_address_wallet;

                ALTER TABLE wallet
                    DROP CONSTRAINT uq_wallet_account_id;

                CREATE UNIQUE INDEX idx_wallet_account_id
                    ON wallet(account_id, id);

                ALTER TABLE ln_address
                    ADD CONSTRAINT fk_ln_address_wallet
                    FOREIGN KEY (account_id, wallet_id)
                    REFERENCES wallet(account_id, id)
                    ON DELETE CASCADE;
                "#,
            )
            .await?;

        Ok(())
    }
}
