use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Use `execute_unprepared` if the SQL statement doesn't have value bindings
        db.execute_unprepared(
            "CREATE OR REPLACE FUNCTION update_timestamp() RETURNS TRIGGER AS $$ BEGIN NEW.updated_at = NOW();
            RETURN NEW;
            END;
            $$ language 'plpgsql';
            CREATE TABLE lightning_address (
                id uuid primary key default gen_random_uuid(),
                user_id varchar(255) unique not null,
                username varchar(255) unique not null,
                active boolean not null default true,
                created_at timestamptz not null default current_timestamp,
                updated_at timestamptz,
                deleted_at timestamptz
            );
            CREATE TRIGGER update_timestamp BEFORE
            UPDATE ON lightning_address FOR EACH ROW EXECUTE PROCEDURE update_timestamp();",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE `lightning_address`")
            .await?;

        Ok(())
    }
}
