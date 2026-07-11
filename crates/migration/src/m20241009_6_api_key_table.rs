use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260704_000001_account_table::Account;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table
            .table(ApiKey::Table)
            .if_not_exists()
            .col(uuid(ApiKey::Id).primary_key())
            .col(string_len(ApiKey::Name, 255))
            .col(binary_len_uniq(ApiKey::KeyHash, 32))
            .col(json(ApiKey::Permissions))
            .col(text_null(ApiKey::Description))
            .col(timestamp(ApiKey::CreatedAt).default(Expr::current_timestamp()))
            .col(timestamp_null(ApiKey::ExpiresAt));

        if manager.get_database_backend() == DatabaseBackend::Sqlite {
            table.col(uuid(ApiKey::AccountId)).foreign_key(
                ForeignKey::create()
                    .name("fk_api_key_account")
                    .from(ApiKey::Table, ApiKey::AccountId)
                    .to(Account::Table, Account::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            );
        }

        manager.create_table(table.to_owned()).await?;

        if manager.get_database_backend() == DatabaseBackend::Sqlite {
            manager
                .create_index(
                    Index::create()
                        .name("idx_api_key_account_id")
                        .table(ApiKey::Table)
                        .col(ApiKey::AccountId)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ApiKey::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub(crate) enum ApiKey {
    Table,
    Id,
    Name,
    KeyHash,
    Permissions,
    Description,
    CreatedAt,
    ExpiresAt,
    AccountId,
}
