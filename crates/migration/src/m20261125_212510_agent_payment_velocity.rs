use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum SpendingPolicy {
    Table,
    AgentHourlyMsat,
}
#[derive(DeriveIden)]
enum SpendingEntry {
    Table,
    AccountId,
    AgentKeyId,
    AdmittedAt,
}

const INDEX: &str = "idx_spending_entry_agent_hour";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SpendingPolicy::Table)
                    .add_column(ColumnDef::new(SpendingPolicy::AgentHourlyMsat).big_integer())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name(INDEX)
                    .table(SpendingEntry::Table)
                    .col(SpendingEntry::AccountId)
                    .col(SpendingEntry::AgentKeyId)
                    .col(SpendingEntry::AdmittedAt)
                    .and_where(Expr::col(SpendingEntry::AgentKeyId).is_not_null())
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(INDEX).table(SpendingEntry::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(SpendingPolicy::Table)
                    .drop_column(SpendingPolicy::AgentHourlyMsat)
                    .to_owned(),
            )
            .await
    }
}
