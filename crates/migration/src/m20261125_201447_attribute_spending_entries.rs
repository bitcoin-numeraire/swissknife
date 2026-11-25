use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, TransactionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum SpendingEntry {
    Table,
    WalletId,
    AgentKeyId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let tx = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&tx);
        for column in [SpendingEntry::WalletId, SpendingEntry::AgentKeyId] {
            schema
                .alter_table(
                    Table::alter()
                        .table(SpendingEntry::Table)
                        .add_column(ColumnDef::new(column).uuid())
                        .to_owned(),
                )
                .await?;
        }
        // Prior versions did not record the authorizing credential. The wallet is
        // recoverable only while its original payment still exists.
        tx.execute_unprepared("UPDATE spending_entry SET wallet_id = (SELECT wallet_id FROM payment WHERE payment.id = spending_entry.payment_id)").await?;
        tx.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let tx = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&tx);
        for column in [SpendingEntry::AgentKeyId, SpendingEntry::WalletId] {
            schema
                .alter_table(
                    Table::alter()
                        .table(SpendingEntry::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }
        tx.commit().await
    }
}
