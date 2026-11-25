use sea_orm_migration::{prelude::*, sea_orm::TransactionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum SpendingPolicy {
    Table,
    AccountId,
    Enabled,
    PerPaymentMsat,
    DailyMsat,
    WeeklyMsat,
    MonthlyMsat,
}

#[derive(DeriveIden)]
enum SpendingEntry {
    Table,
    PaymentId,
    AccountId,
    AdmittedAt,
    ChargedMsat,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&txn);
        use SpendingEntry as E;
        use SpendingPolicy as P;
        schema
            .create_table(
                Table::create()
                    .table(P::Table)
                    .col(ColumnDef::new(P::AccountId).uuid().primary_key())
                    .col(ColumnDef::new(P::Enabled).boolean().not_null())
                    .col(ColumnDef::new(P::PerPaymentMsat).big_integer())
                    .col(ColumnDef::new(P::DailyMsat).big_integer())
                    .col(ColumnDef::new(P::WeeklyMsat).big_integer())
                    .col(ColumnDef::new(P::MonthlyMsat).big_integer())
                    .foreign_key(
                        ForeignKey::create()
                            .from(P::Table, P::AccountId)
                            .to(Alias::new("account"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        // Retain consumption when an operator deletes a payment or wallet. Only deleting
        // the account discards its budget history. Each payment has exactly one entry.
        schema
            .create_table(
                Table::create()
                    .table(E::Table)
                    .col(ColumnDef::new(E::PaymentId).uuid().primary_key())
                    .col(ColumnDef::new(E::AccountId).uuid().not_null())
                    .col(ColumnDef::new(E::AdmittedAt).date_time().not_null())
                    .col(ColumnDef::new(E::ChargedMsat).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(E::Table, E::AccountId)
                            .to(P::Table, P::AccountId)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        schema
            .create_index(
                Index::create()
                    .name("idx_spending_entry_account_time")
                    .table(E::Table)
                    .col(E::AccountId)
                    .col(E::AdmittedAt)
                    .to_owned(),
            )
            .await?;
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&txn);
        schema
            .drop_table(Table::drop().table(SpendingEntry::Table).to_owned())
            .await?;
        schema
            .drop_table(Table::drop().table(SpendingPolicy::Table).to_owned())
            .await?;
        txn.commit().await
    }
}
