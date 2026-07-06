use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::{prelude::*, schema::*};

use crate::m20241009_6_api_key_table::ApiKey;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ApiKey::Table)
                    .add_column(uuid_null(ApiKey::AccountId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_api_key_account_id")
                    .table(ApiKey::Table)
                    .col(ApiKey::AccountId)
                    .to_owned(),
            )
            .await?;

        if manager.get_database_backend() == DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute(Statement::from_string(
                    DatabaseBackend::Postgres,
                    format!(
                        "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL",
                        ApiKey::Table.to_string(),
                        ApiKey::AccountId.to_string()
                    ),
                ))
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DatabaseBackend::Postgres {
            manager
                .get_connection()
                .execute(Statement::from_string(
                    DatabaseBackend::Postgres,
                    format!(
                        "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL",
                        ApiKey::Table.to_string(),
                        ApiKey::AccountId.to_string()
                    ),
                ))
                .await?;
        }

        manager
            .drop_index(
                Index::drop()
                    .name("idx_api_key_account_id")
                    .table(ApiKey::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ApiKey::Table)
                    .drop_column(ApiKey::AccountId)
                    .to_owned(),
            )
            .await
    }
}
