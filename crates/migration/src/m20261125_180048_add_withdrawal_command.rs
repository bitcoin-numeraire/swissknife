use sea_orm_migration::{prelude::*, sea_orm::TransactionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        use WithdrawalCommand::*;
        let transaction = manager.get_connection().begin().await?;
        let manager = SchemaManager::new(&transaction);
        let db = manager.get_connection();
        db.execute_unprepared("DROP INDEX idx_client_event_type_resource")
            .await?;
        // Terminal events stay unique. A withdrawal can wait or re-enter reconciliation more than once.
        db.execute_unprepared("CREATE UNIQUE INDEX idx_client_event_type_resource ON client_event(event_type,resource_id)
            WHERE event_type NOT IN ('withdrawal.processing','withdrawal.waiting','withdrawal.broadcast','withdrawal.reconciling')").await?;
        manager
            .create_table(
                Table::create()
                    .table(TableName)
                    .col(ColumnDef::new(PaymentId).uuid().primary_key())
                    .col(ColumnDef::new(Backend).string().not_null())
                    .col(ColumnDef::new(Address).string().not_null())
                    .col(ColumnDef::new(MaximumFeeMsat).big_integer().not_null())
                    .col(ColumnDef::new(ExpiresAt).date_time().not_null())
                    .col(ColumnDef::new(Progress).json_binary().not_null())
                    .col(ColumnDef::new(StartedAt).date_time())
                    .col(ColumnDef::new(ClaimToken).uuid())
                    .col(ColumnDef::new(NextAttempt).date_time())
                    .foreign_key(
                        ForeignKey::create()
                            .from(TableName, PaymentId)
                            .to(Alias::new("payment"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_withdrawal_command_ready")
                    .table(TableName)
                    .col(Backend)
                    .col(NextAttempt)
                    .to_owned(),
            )
            .await?;
        transaction.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let transaction = manager.get_connection().begin().await?;
        let manager = SchemaManager::new(&transaction);
        let db = manager.get_connection();
        db.execute_unprepared("DELETE FROM client_event WHERE event_type IN ('withdrawal.processing','withdrawal.waiting','withdrawal.broadcast','withdrawal.reconciling')").await?;
        db.execute_unprepared("DROP INDEX idx_client_event_type_resource")
            .await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_client_event_type_resource ON client_event(event_type,resource_id)",
        )
        .await?;
        manager
            .drop_table(Table::drop().table(WithdrawalCommand::TableName).to_owned())
            .await?;
        transaction.commit().await
    }
}

#[derive(DeriveIden)]
enum WithdrawalCommand {
    #[sea_orm(iden = "withdrawal_command")]
    TableName,
    PaymentId,
    Backend,
    Address,
    MaximumFeeMsat,
    ExpiresAt,
    Progress,
    StartedAt,
    ClaimToken,
    NextAttempt,
}
