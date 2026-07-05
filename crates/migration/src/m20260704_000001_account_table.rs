use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Account::Table)
                    .if_not_exists()
                    .col(uuid(Account::Id).primary_key())
                    .col(text_null(Account::DisplayName))
                    .col(timestamp(Account::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(Account::UpdatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Account::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Account {
    Table,
    Id,
    DisplayName,
    CreatedAt,
    UpdatedAt,
}
