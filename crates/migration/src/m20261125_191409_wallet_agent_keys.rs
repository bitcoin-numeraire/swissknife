use sea_orm_migration::{prelude::*, sea_orm::TransactionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum WalletAgentKey {
    Table,
    Id,
    WalletId,
    AccountId,
    Name,
    KeyHash,
    CanRead,
    CanReceive,
    CanSpend,
    CreatedAt,
    ExpiresAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        use WalletAgentKey::*;
        let txn = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&txn);
        schema
            .create_table(
                sea_orm_migration::prelude::Table::create()
                    .table(Table)
                    .col(ColumnDef::new(Id).uuid().primary_key())
                    .col(ColumnDef::new(WalletId).uuid().not_null())
                    .col(ColumnDef::new(AccountId).uuid().not_null())
                    .col(ColumnDef::new(Name).string().not_null())
                    .col(ColumnDef::new(KeyHash).binary().unique_key().not_null())
                    .col(ColumnDef::new(CanRead).boolean().not_null())
                    .col(ColumnDef::new(CanReceive).boolean().not_null())
                    .col(ColumnDef::new(CanSpend).boolean().not_null())
                    .col(ColumnDef::new(CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(ExpiresAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Table, (WalletId, AccountId))
                            .to(Alias::new("wallet"), (Alias::new("id"), Alias::new("account_id")))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        schema
            .create_index(
                Index::create()
                    .name("idx_wallet_agent_key_wallet")
                    .table(Table)
                    .col(WalletId)
                    .to_owned(),
            )
            .await?;
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WalletAgentKey::Table).to_owned())
            .await
    }
}
