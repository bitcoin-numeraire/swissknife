use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Payment {
    Table,
    Ledger,
    Status,
    Id,
}

const INDEX: &str = "idx_payment_ledger_status_id";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name(INDEX)
                    .table(Payment::Table)
                    .col(Payment::Ledger)
                    .col(Payment::Status)
                    .col(Payment::Id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(INDEX).table(Payment::Table).to_owned())
            .await
    }
}
