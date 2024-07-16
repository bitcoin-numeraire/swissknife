use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            r#"
            CREATE TABLE ln_address (
                id UUID PRIMARY KEY DEFAULT gen_random_UUID(),
                user_id UUID UNIQUE NOT NULL,
                username VARCHAR(255) UNIQUE NOT NULL,
                active boolean NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
                updated_at TIMESTAMPTZ,
                CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES account (id) ON DELETE CASCADE
            );

            CREATE OR REPLACE FUNCTION update_timestamp() RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = NOW();
                RETURN NEW;
            END;
            $$ language 'plpgsql';

            CREATE TRIGGER update_timestamp BEFORE UPDATE ON ln_address FOR EACH ROW EXECUTE PROCEDURE update_timestamp();
            "#
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE ln_address")
            .await?;

        Ok(())
    }
}
