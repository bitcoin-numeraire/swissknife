use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKey::Table)
                    .if_not_exists()
                    .col(uuid(ApiKey::Id).primary_key())
                    .col(string_len(ApiKey::UserId, 255))
                    .col(string_len(ApiKey::Name, 255))
                    .col(binary_len_uniq(ApiKey::KeyHash, 32))
                    .col(json(ApiKey::Permissions))
                    .col(text_null(ApiKey::Description))
                    .col(timestamp(ApiKey::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(ApiKey::ExpiresAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ApiKey::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub(crate) enum ApiKey {
    Table,
    Id,
    UserId,
    Name,
    KeyHash,
    Permissions,
    Description,
    CreatedAt,
    ExpiresAt,
    AccountId,
}
