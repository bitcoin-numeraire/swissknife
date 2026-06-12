use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240420_1_wallet_table::Wallet;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BtcAddress::Table)
                    .if_not_exists()
                    .col(uuid(BtcAddress::Id).primary_key())
                    .col(uuid(BtcAddress::WalletId))
                    .col(string_len(BtcAddress::Address, 255).unique_key())
                    .col(string_len(BtcAddress::AddressType, 255))
                    .col(boolean(BtcAddress::Used).default(false))
                    .col(timestamp(BtcAddress::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(BtcAddress::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_btc_address_wallet")
                            .from(BtcAddress::Table, BtcAddress::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_btc_address_wallet_used")
                    .table(BtcAddress::Table)
                    .col(BtcAddress::WalletId)
                    .col(BtcAddress::Used)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_btc_address_wallet_used")
                    .table(BtcAddress::Table)
                    .to_owned(),
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(BtcAddress::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum BtcAddress {
    Table,
    Id,
    WalletId,
    Address,
    AddressType,
    Used,
    CreatedAt,
    UpdatedAt,
}
