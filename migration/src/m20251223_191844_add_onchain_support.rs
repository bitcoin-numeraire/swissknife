use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_1_wallet_table::Wallet, m20240420_3_invoice_table::Invoice, m20240420_4_payment_table::Payment};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create onchain_address table
        manager
            .create_table(
                Table::create()
                    .table(OnchainAddress::Table)
                    .if_not_exists()
                    .col(uuid(OnchainAddress::Id).primary_key())
                    .col(uuid(OnchainAddress::WalletId))
                    .col(string(OnchainAddress::Address).unique_key())
                    .col(boolean(OnchainAddress::Used).default(false))
                    .col(big_integer_null(OnchainAddress::DerivationIndex))
                    .col(timestamp(OnchainAddress::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(OnchainAddress::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wallet")
                            .from(OnchainAddress::Table, OnchainAddress::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_wallet_used")
                            .table(OnchainAddress::Table)
                            .col(OnchainAddress::WalletId)
                            .col(OnchainAddress::Used),
                    )
                    .to_owned(),
            )
            .await?;

        // Add onchain-specific fields to invoice table
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .add_column(string_null(Invoice::Txid))
                    .add_column(integer_null(Invoice::Confirmations).default(0))
                    .add_column(integer_null(Invoice::OutputIndex))
                    .to_owned(),
            )
            .await?;

        // Add unique index on txid
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("UNIQUE_txid")
                    .table(Invoice::Table)
                    .col(Invoice::Txid)
                    .nulls_not_distinct()
                    .to_owned(),
            )
            .await?;

        // Add onchain-specific fields to payment table
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(string_null(Payment::Txid))
                    .add_column(integer_null(Payment::Confirmations).default(0))
                    .add_column(string_null(Payment::DestinationAddress))
                    .to_owned(),
            )
            .await?;

        // Add unique index on payment txid
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("UNIQUE_payment_txid")
                    .table(Payment::Table)
                    .col(Payment::Txid)
                    .nulls_not_distinct()
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
                    .name("UNIQUE_txid")
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
                    .drop_column(Payment::Confirmations)
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
                    .drop_column(Invoice::Confirmations)
                    .drop_column(Invoice::Txid)
                    .to_owned(),
            )
            .await?;

        // Drop onchain_address table
        manager
            .drop_table(Table::drop().table(OnchainAddress::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum OnchainAddress {
    Table,
    Id,
    WalletId,
    Address,
    Used,
    DerivationIndex,
    CreatedAt,
    UpdatedAt,
}
