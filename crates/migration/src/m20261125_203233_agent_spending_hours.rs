use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum AgentSpendingHours {
    Table,
    AccountId,
    Weekdays,
    StartMinute,
    EndMinute,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        use AgentSpendingHours as H;
        manager
            .create_table(
                Table::create()
                    .table(H::Table)
                    .col(ColumnDef::new(H::AccountId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(H::Weekdays).small_integer().not_null())
                    .col(ColumnDef::new(H::StartMinute).small_integer().not_null())
                    .col(ColumnDef::new(H::EndMinute).small_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(H::Table, H::AccountId)
                            .to(Alias::new("account"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AgentSpendingHours::Table).to_owned())
            .await
    }
}
