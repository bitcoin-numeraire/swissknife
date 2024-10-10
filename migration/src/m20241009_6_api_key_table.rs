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
            CREATE TABLE api_key (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id VARCHAR(255) NOT NULL,
                key_hash BYTEA UNIQUE NOT NULL,
                permissions TEXT[] NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
                expires_at TIMESTAMPTZ,
                description TEXT,
                FOREIGN KEY (user_id) REFERENCES wallet(user_id) ON DELETE CASCADE
            );
            "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE api_key")
            .await?;

        Ok(())
    }
}
