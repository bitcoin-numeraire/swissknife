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
                    .table(Payment::Table)
                    .if_not_exists()
                    .col(uuid(Payment::Id).primary_key())
                    .col(uuid(Payment::WalletId))
                    .col(string_len_null(Payment::LnAddress, 255))
                    .col(string_len_null(Payment::PaymentHash, 255).unique_key())
                    .col(string_len_null(Payment::PaymentPreimage, 255).unique_key())
                    .col(string_null(Payment::Error))
                    .col(big_integer(Payment::AmountMsat))
                    .col(big_integer_null(Payment::FeeMsat))
                    .col(timestamp_null(Payment::PaymentTime))
                    .col(string(Payment::Status))
                    .col(string_len(Payment::Ledger, 255))
                    .col(string_len(Payment::Currency, 255))
                    .col(string_null(Payment::Description))
                    .col(string_null(Payment::Metadata))
                    .col(json_binary_null(Payment::SuccessAction))
                    .col(timestamp(Payment::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(Payment::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wallet")
                            .from(Payment::Table, Payment::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Payment::Table).to_owned())
            .await?;

        manager
            .drop_foreign_key(ForeignKey::drop().name("fk_wallet").table(Payment::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum Payment {
    Table,
    Id,
    WalletId,
    LnAddress,
    PaymentHash,
    PaymentPreimage,
    Error,
    AmountMsat,
    FeeMsat,
    PaymentTime,
    Status,
    Ledger,
    Currency,
    Description,
    Metadata,
    SuccessAction,
    CreatedAt,
    UpdatedAt,
    // Bitcoin-specific fields
    Txid,
    Confirmed,
    DestinationAddress,
}
