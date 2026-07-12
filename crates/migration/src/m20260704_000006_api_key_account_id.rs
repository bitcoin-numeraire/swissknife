use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20241009_6_api_key_table::ApiKey, m20260704_000001_account_table::Account};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        manager
            .alter_table(
                Table::alter()
                    .table(ApiKey::Table)
                    .add_column(uuid(ApiKey::AccountId))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ApiKey::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_api_key_account")
                            .from_tbl(ApiKey::Table)
                            .from_col(ApiKey::AccountId)
                            .to_tbl(Account::Table)
                            .to_col(Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
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
                    .drop_foreign_key(Alias::new("fk_api_key_account"))
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
