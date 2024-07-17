use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            r#"
            CREATE TABLE wallet (
                id UUID PRIMARY KEY DEFAULT gen_random_UUID(),
                user_id VARCHAR(255) UNIQUE NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
                updated_at TIMESTAMPTZ
            );

            CREATE OR REPLACE FUNCTION update_wallet_timestamp() RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = NOW();
                RETURN NEW;
            END;
            $$ language 'plpgsql';

            CREATE TRIGGER update_wallet_timestamp BEFORE UPDATE ON wallet FOR EACH ROW EXECUTE PROCEDURE update_wallet_timestamp();
            "#
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE wallet")
            .await?;

        Ok(())
    }
}
