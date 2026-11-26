use sea_orm_migration::{prelude::*, schema::*};

use super::m20240420_000003_invoice_table::Invoice;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Receipt {
    InternalBtcAddress,
}

const ADDRESS_INDEX: &str = "idx-invoice-internal-btc-address";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .add_column(string_null(Receipt::InternalBtcAddress))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name(ADDRESS_INDEX)
                    .table(Invoice::Table)
                    .col(Receipt::InternalBtcAddress)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(ADDRESS_INDEX).table(Invoice::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .drop_column(Receipt::InternalBtcAddress)
                    .to_owned(),
            )
            .await
    }
}
