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

        // SeaQuery used to emit INTEGER for unsigned 32-bit columns on Postgres.
        // Fresh schemas now get BIGINT; existing schemas must match the i64 models.
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE btc_output
                    ALTER COLUMN output_index TYPE BIGINT,
                    ALTER COLUMN block_height TYPE BIGINT;
                 ALTER TABLE payment ALTER COLUMN btc_block_height TYPE BIGINT;",
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
                "ALTER TABLE btc_output
                    ALTER COLUMN output_index TYPE INTEGER,
                    ALTER COLUMN block_height TYPE INTEGER;
                 ALTER TABLE payment ALTER COLUMN btc_block_height TYPE INTEGER;",
            )
            .await?;
        Ok(())
    }
}
