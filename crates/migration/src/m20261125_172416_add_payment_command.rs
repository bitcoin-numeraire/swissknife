use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentCommand::Table)
                    .col(ColumnDef::new(PaymentCommand::Id).uuid().primary_key())
                    .col(ColumnDef::new(PaymentCommand::WalletId).uuid().not_null())
                    .col(ColumnDef::new(PaymentCommand::IdempotencyKey).uuid().not_null())
                    .col(ColumnDef::new(PaymentCommand::PaymentId).uuid().unique_key())
                    .col(ColumnDef::new(PaymentCommand::RequestFingerprint).string().not_null())
                    .col(ColumnDef::new(PaymentCommand::Instructions).json_binary().not_null())
                    .col(ColumnDef::new(PaymentCommand::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(PaymentCommand::DispatchedAt).date_time())
                    .foreign_key(
                        ForeignKey::create()
                            .from(PaymentCommand::Table, PaymentCommand::WalletId)
                            .to(Alias::new("wallet"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(PaymentCommand::Table, PaymentCommand::PaymentId)
                            .to(Alias::new("payment"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("idx_payment_command_key")
                    .table(PaymentCommand::Table)
                    .col(PaymentCommand::WalletId)
                    .col(PaymentCommand::IdempotencyKey)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_payment_command_queue")
                    .table(PaymentCommand::Table)
                    .col(PaymentCommand::DispatchedAt)
                    .col(PaymentCommand::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PaymentCommand::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PaymentCommand {
    Table,
    Id,
    WalletId,
    IdempotencyKey,
    PaymentId,
    RequestFingerprint,
    Instructions,
    CreatedAt,
    DispatchedAt,
}
