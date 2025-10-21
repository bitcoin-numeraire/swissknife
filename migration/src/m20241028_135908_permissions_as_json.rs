use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::m20241009_6_api_key_table::ApiKey;

#[derive(DeriveMigrationName)]
pub struct Migration;

// This migration is specific to Postgres where the "permissions" column was defined as Array and needs to be moved to JSON
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();

        if db_backend == DatabaseBackend::Postgres {
            manager
                .alter_table(
                    Table::alter()
                        .table(ApiKey::Table)
                        .drop_column(ApiKey::Permissions)
                        .to_owned(),
                )
                .await?;

            manager
                .alter_table(
                    Table::alter()
                        .table(ApiKey::Table)
                        .add_column(json(ApiKey::Permissions))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
