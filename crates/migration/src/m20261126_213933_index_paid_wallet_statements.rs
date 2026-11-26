use sea_orm_migration::prelude::*;

use crate::{m20240420_000003_invoice_table::Invoice, m20240420_000004_payment_table::Payment};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_invoice_wallet_payment_time")
                    .table(Invoice::Table)
                    .col(Invoice::WalletId)
                    .col(Invoice::PaymentTime)
                    .and_where(Expr::col(Invoice::PaymentTime).is_not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_payment_wallet_payment_time")
                    .table(Payment::Table)
                    .col(Payment::WalletId)
                    .col(Payment::PaymentTime)
                    .and_where(Expr::col(Payment::PaymentTime).is_not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in ["idx_payment_wallet_payment_time", "idx_invoice_wallet_payment_time"] {
            manager.drop_index(Index::drop().name(name).to_owned()).await?;
        }
        Ok(())
    }
}
