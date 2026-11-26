use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum SpendingPolicy {
    Table,
    AgentHourlyWarningMsat,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SpendingPolicy::Table)
                    .add_column(ColumnDef::new(SpendingPolicy::AgentHourlyWarningMsat).big_integer())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SpendingPolicy::Table)
                    .drop_column(SpendingPolicy::AgentHourlyWarningMsat)
                    .to_owned(),
            )
            .await
    }
}
