use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum L402Usage {
    Table,
    PaymentHash,
    WalletId,
    Route,
    Requests,
    FirstUsedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(L402Usage::Table)
                    .col(
                        ColumnDef::new(L402Usage::PaymentHash)
                            .string_len(64)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(L402Usage::WalletId).uuid().not_null())
                    .col(ColumnDef::new(L402Usage::Route).string_len(64).not_null())
                    .col(ColumnDef::new(L402Usage::Requests).big_integer().not_null())
                    .col(ColumnDef::new(L402Usage::FirstUsedAt).date_time().not_null())
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(L402Usage::Table).to_owned())
            .await
    }
}
