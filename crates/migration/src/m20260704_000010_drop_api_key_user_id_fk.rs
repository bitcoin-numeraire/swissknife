use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        match db.get_database_backend() {
            DatabaseBackend::Postgres => db
                .execute(Statement::from_string(
                    DatabaseBackend::Postgres,
                    "ALTER TABLE api_key DROP CONSTRAINT IF EXISTS api_key_user_id_fkey".to_string(),
                ))
                .await
                .map(|_| ()),
            DatabaseBackend::Sqlite | DatabaseBackend::MySql => Ok(()),
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
