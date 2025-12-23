use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_1_wallet_table::Wallet, m20240420_3_invoice_table::Invoice, m20240420_4_payment_table::Payment};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create btc_address table
        manager
            .create_table(
                Table::create()
                    .table(BtcAddress::Table)
                    .if_not_exists()
                    .col(uuid(BtcAddress::Id).primary_key())
                    .col(uuid(BtcAddress::WalletId))
                    .col(string(BtcAddress::Address).unique_key())
                    .col(boolean(BtcAddress::Used).default(false))
                    .col(big_integer_null(BtcAddress::DerivationIndex))
                    .col(timestamp(BtcAddress::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(BtcAddress::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_btc_address_wallet")
                            .from(BtcAddress::Table, BtcAddress::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_btc_address_wallet_used")
                            .table(BtcAddress::Table)
                            .col(BtcAddress::WalletId)
                            .col(BtcAddress::Used),
                    )
                    .to_owned(),
            )
            .await?;

        // Add Bitcoin-specific fields to invoice table
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .add_column(string_null(Invoice::Txid))
                    .add_column(boolean_null(Invoice::Confirmed).default(false))
                    .add_column(integer_null(Invoice::OutputIndex))
                    .to_owned(),
            )
            .await?;

        // Add unique index on txid (allowing multiple NULLs for Lightning invoices)
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("UNIQUE_invoice_txid")
                    .table(Invoice::Table)
                    .col(Invoice::Txid)
                    .to_owned(),
            )
            .await?;

        // Add Bitcoin-specific fields to payment table
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(string_null(Payment::Txid))
                    .add_column(boolean_null(Payment::Confirmed).default(false))
                    .add_column(string_null(Payment::DestinationAddress))
                    .to_owned(),
            )
            .await?;

        // Add unique index on payment txid (allowing multiple NULLs for Lightning payments)
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("UNIQUE_payment_txid")
                    .table(Payment::Table)
                    .col(Payment::Txid)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes
        manager
            .drop_index(
                Index::drop()
                    .name("UNIQUE_payment_txid")
                    .table(Payment::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("UNIQUE_invoice_txid")
                    .table(Invoice::Table)
                    .to_owned(),
            )
            .await?;

        // Remove fields from payment table
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::DestinationAddress)
                    .drop_column(Payment::Confirmed)
                    .drop_column(Payment::Txid)
                    .to_owned(),
            )
            .await?;

        // Remove fields from invoice table
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .drop_column(Invoice::OutputIndex)
                    .drop_column(Invoice::Confirmed)
                    .drop_column(Invoice::Txid)
                    .to_owned(),
            )
            .await?;

        // Drop btc_address table
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
    Used,
    DerivationIndex,
    CreatedAt,
    UpdatedAt,
}
