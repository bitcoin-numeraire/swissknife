use sea_orm_migration::{prelude::*, schema::*};

use crate::{
    m20240420_1_wallet_table::Wallet, m20240420_3_invoice_table::Invoice, m20240420_4_payment_table::Payment,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BitcoinAddress::Table)
                    .if_not_exists()
                    .col(uuid(BitcoinAddress::Id).primary_key())
                    .col(uuid(BitcoinAddress::WalletId))
                    .col(string_len(BitcoinAddress::Address, 255))
                    .col(boolean(BitcoinAddress::Used).default(false))
                    .col(integer_null(BitcoinAddress::DerivationIndex))
                    .col(timestamp(BitcoinAddress::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(BitcoinAddress::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_bitcoin_address_wallet")
                            .from(BitcoinAddress::Table, BitcoinAddress::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("UNIQUE_bitcoin_address")
                            .table(BitcoinAddress::Table)
                            .col(BitcoinAddress::Address)
                            .unique(),
                    )
                    .index(
                        Index::create()
                            .name("INDEX_bitcoin_address_wallet_used")
                            .table(BitcoinAddress::Table)
                            .col(BitcoinAddress::WalletId)
                            .col(BitcoinAddress::Used),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(BitcoinTransaction::Table)
                    .if_not_exists()
                    .col(uuid(BitcoinTransaction::Id).primary_key())
                    .col(string_len(BitcoinTransaction::Txid, 255))
                    .col(big_integer(BitcoinTransaction::AmountSat))
                    .col(big_integer_null(BitcoinTransaction::FeeSat))
                    .col(integer_null(BitcoinTransaction::BlockHeight))
                    .col(timestamp_null(BitcoinTransaction::Timestamp))
                    .col(string_len(BitcoinTransaction::Currency, 255))
                    .col(timestamp(BitcoinTransaction::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(BitcoinTransaction::UpdatedAt))
                    .index(
                        Index::create()
                            .name("UNIQUE_bitcoin_txid")
                            .table(BitcoinTransaction::Table)
                            .col(BitcoinTransaction::Txid)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table({
                let fk_invoice = TableForeignKey::new()
                    .name("fk_invoice_btc_transaction")
                    .from_tbl(Invoice::Table)
                    .from_col(Alias::new("btc_transaction_id"))
                    .to_tbl(BitcoinTransaction::Table)
                    .to_col(BitcoinTransaction::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned();

                Table::alter()
                    .table(Invoice::Table)
                    .add_column(uuid_null(Alias::new("btc_transaction_id")))
                    .add_column(integer_null(Alias::new("output_index")))
                    .add_foreign_key(&fk_invoice)
                    .to_owned()
            })
            .await?;

        manager
            .alter_table({
                let fk_payment = TableForeignKey::new()
                    .name("fk_payment_btc_transaction")
                    .from_tbl(Payment::Table)
                    .from_col(Alias::new("btc_transaction_id"))
                    .to_tbl(BitcoinTransaction::Table)
                    .to_col(BitcoinTransaction::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned();

                Table::alter()
                    .table(Payment::Table)
                    .add_column(uuid_null(Alias::new("btc_transaction_id")))
                    .add_column(string_len_null(Alias::new("destination_address"), 255))
                    .add_foreign_key(&fk_payment)
                    .to_owned()
            })
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_foreign_key(Alias::new("fk_payment_btc_transaction"))
                    .drop_column(Alias::new("btc_transaction_id"))
                    .drop_column(Alias::new("destination_address"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .drop_foreign_key(Alias::new("fk_invoice_btc_transaction"))
                    .drop_column(Alias::new("btc_transaction_id"))
                    .drop_column(Alias::new("output_index"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(BitcoinTransaction::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(BitcoinAddress::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum BitcoinAddress {
    Table,
    Id,
    WalletId,
    Address,
    Used,
    DerivationIndex,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum BitcoinTransaction {
    Table,
    Id,
    Txid,
    AmountSat,
    FeeSat,
    BlockHeight,
    Timestamp,
    Currency,
    CreatedAt,
    UpdatedAt,
}
