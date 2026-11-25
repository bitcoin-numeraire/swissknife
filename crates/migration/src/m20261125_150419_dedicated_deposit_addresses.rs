use sea_orm_migration::{prelude::*, schema::*};

use super::m20251224_162538_btc_address_table::BtcAddress;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Allocation {
    Dedicated,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(BtcAddress::Table)
                    .add_column(boolean(Allocation::Dedicated).default(false))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(BtcAddress::Table)
                    .drop_column(Allocation::Dedicated)
                    .to_owned(),
            )
            .await
    }
}
