use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240420_4_payment_table::Payment;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(json_binary_null(Payment::RawSuccessAction))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::RawSuccessAction)
                    .to_owned(),
            )
            .await
    }
}
