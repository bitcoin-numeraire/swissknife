use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE ln_address
                ADD COLUMN allows_nostr BOOLEAN NOT NULL DEFAULT false,
                ADD COLUMN nostr_pubkey VARCHAR(255);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE ln_address
                DROP COLUMN allows_nostr,
                DROP COLUMN nostr_pubkey;
                "#,
            )
            .await?;

        Ok(())
    }
}
