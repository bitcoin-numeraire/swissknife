use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO config (key, value)
             SELECT 'replay_legacy_lnd_withdrawals', 'true'
             WHERE EXISTS (SELECT 1 FROM payment WHERE ledger = 'Onchain'
                         AND status = 'Pending' AND payment_hash IS NOT NULL)
             ON CONFLICT (key) DO NOTHING",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DELETE FROM config WHERE key = 'replay_legacy_lnd_withdrawals'")
            .await?;
        Ok(())
    }
}
