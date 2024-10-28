use sea_orm_migration::{
    prelude::*,
    schema::{boolean, string_len_null},
};

use crate::m20240420_2_ln_address_table::LnAddress;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LnAddress::Table)
                    .add_column(boolean(LnAddress::AllowsNostr).default(false))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(LnAddress::Table)
                    .add_column(string_len_null(LnAddress::NostrPubkey, 255))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(LnAddress::Table)
                    .drop_column(LnAddress::AllowsNostr)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(LnAddress::Table)
                    .drop_column(LnAddress::NostrPubkey)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
