use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240420_4_payment_table::Payment;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add destination_address column
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(string_len_null(Payment::BtcAddress, 255))
                    .to_owned(),
            )
            .await?;

        // Add block height column
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(unsigned_null(Payment::BtcBlockHeight))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop columns
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::BtcBlockHeight)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::BtcAddress)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
