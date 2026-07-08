use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        match db.get_database_backend() {
            DatabaseBackend::Postgres => {
                execute(
                    db,
                    DatabaseBackend::Postgres,
                    "ALTER TABLE api_key DROP CONSTRAINT IF EXISTS api_key_user_id_fkey",
                )
                .await?;
                execute(
                    db,
                    DatabaseBackend::Postgres,
                    "ALTER TABLE api_key DROP COLUMN IF EXISTS user_id",
                )
                .await?;
                execute(
                    db,
                    DatabaseBackend::Postgres,
                    "ALTER TABLE wallet DROP COLUMN IF EXISTS user_id",
                )
                .await
            }
            DatabaseBackend::Sqlite => {
                execute(db, DatabaseBackend::Sqlite, "ALTER TABLE wallet DROP COLUMN user_id").await
            }
            DatabaseBackend::MySql => Ok(()),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn execute(db: &dyn ConnectionTrait, backend: DatabaseBackend, sql: &str) -> Result<(), DbErr> {
    db.execute(Statement::from_string(backend, sql.to_string()))
        .await
        .map(|_| ())
}
