use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Wallet::Table)
                    .if_not_exists()
                    .col(uuid(Wallet::Id).primary_key())
                    .col(string_len_uniq(Wallet::UserId, 255))
                    .col(timestamp(Wallet::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(Wallet::UpdatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Wallet::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Wallet {
    Table,
    Id,
    UserId,
    CreatedAt,
    UpdatedAt,
}
