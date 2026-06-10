use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_1_wallet_table::Wallet, m20240420_4_payment_table::Payment};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(big_integer(Payment::ReservedAmount).default(0))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WalletBalance::Table)
                    .if_not_exists()
                    .col(uuid(WalletBalance::WalletId))
                    .col(string_len(WalletBalance::Currency, 255))
                    .col(big_integer(WalletBalance::AvailableAmount).default(0))
                    .col(big_integer(WalletBalance::ReservedAmount).default(0))
                    .col(timestamp(WalletBalance::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(WalletBalance::UpdatedAt))
                    .primary_key(
                        Index::create()
                            .name("pk_wallet_balance")
                            .col(WalletBalance::WalletId)
                            .col(WalletBalance::Currency),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wallet_balance_wallet")
                            .from(WalletBalance::Table, WalletBalance::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WalletBalance::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::ReservedAmount)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum WalletBalance {
    Table,
    WalletId,
    Currency,
    AvailableAmount,
    ReservedAmount,
    CreatedAt,
    UpdatedAt,
}
