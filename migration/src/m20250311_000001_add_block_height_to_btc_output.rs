use sea_orm_migration::{prelude::*, schema::*};

use crate::m20251224_162542_btc_output_table::BtcOutput;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(BtcOutput::Table)
                    .add_column_if_not_exists(integer_null(BtcOutput::BlockHeight))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(BtcOutput::Table)
                    .drop_column(BtcOutput::BlockHeight)
                    .to_owned(),
            )
            .await
    }
}
