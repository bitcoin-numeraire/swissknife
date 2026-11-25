use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum MerchantRoute {
    Table,
    WalletId,
    Name,
    Origin,
    PriceMsat,
    Enabled,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        use MerchantRoute::*;
        manager
            .create_table(
                sea_orm_migration::prelude::Table::create()
                    .table(Table)
                    .col(ColumnDef::new(WalletId).uuid().not_null())
                    .col(ColumnDef::new(Name).string_len(57).not_null())
                    .col(ColumnDef::new(Origin).string_len(64).not_null())
                    .col(ColumnDef::new(PriceMsat).big_integer().not_null())
                    .col(ColumnDef::new(Enabled).boolean().not_null())
                    .primary_key(Index::create().col(WalletId).col(Name))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Table, WalletId)
                            .to(Alias::new("wallet"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MerchantRoute::Table).to_owned())
            .await
    }
}
