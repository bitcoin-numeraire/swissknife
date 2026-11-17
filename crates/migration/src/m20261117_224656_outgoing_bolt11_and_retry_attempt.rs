use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(text_null(Payment::Bolt11))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(uuid_null(Payment::AttemptId))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::AttemptId)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::Bolt11)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Payment {
    Table,
    Bolt11,
    AttemptId,
}
