use sea_orm_migration::{
    prelude::*,
    schema::{boolean, string_len_null},
};

use crate::m20240420_000002_ln_address_table::LnAddress;

pub struct Migration;

// Preserve the identifier already recorded in deployed databases.
impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241005_5_ln_address_nostr"
    }
}

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
