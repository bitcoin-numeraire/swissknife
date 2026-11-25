use sea_orm_migration::{prelude::*, sea_orm::TransactionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum SavedContact {
    Table,
    WalletId,
    LnAddress,
    Label,
    SavedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&txn);
        use SavedContact as C;
        schema
            .create_table(
                Table::create()
                    .table(C::Table)
                    .col(ColumnDef::new(C::WalletId).uuid().not_null())
                    .col(ColumnDef::new(C::LnAddress).string().not_null())
                    .col(ColumnDef::new(C::Label).string_len(100))
                    .col(ColumnDef::new(C::SavedAt).date_time().not_null())
                    .primary_key(Index::create().col(C::WalletId).col(C::LnAddress))
                    .foreign_key(
                        ForeignKey::create()
                            .from(C::Table, C::WalletId)
                            .to(Alias::new("wallet"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        schema
            .alter_table(
                Table::alter()
                    .table(Alias::new("wallet_agent_key"))
                    .add_column(
                        ColumnDef::new(Alias::new("can_manage_contacts"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        txn.commit().await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&txn);
        schema
            .alter_table(
                Table::alter()
                    .table(Alias::new("wallet_agent_key"))
                    .drop_column(Alias::new("can_manage_contacts"))
                    .to_owned(),
            )
            .await?;
        schema
            .drop_table(Table::drop().table(SavedContact::Table).to_owned())
            .await?;
        txn.commit().await
    }
}
