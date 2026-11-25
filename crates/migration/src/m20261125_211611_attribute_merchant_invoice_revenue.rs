use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Invoice {
    Table,
    WalletId,
    PaymentTime,
    MerchantRoute,
}

const INDEX: &str = "idx_invoice_merchant_settlement";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .add_column(ColumnDef::new(Invoice::MerchantRoute).string_len(64))
                    .to_owned(),
            )
            .await?;
        // Older request packs have authoritative attribution after their first use.
        // Free-text descriptions on ordinary invoices are buyer-controlled.
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE invoice SET merchant_route = usage.route FROM l402_usage AS usage
             WHERE invoice.payment_hash = usage.payment_hash AND invoice.wallet_id = usage.wallet_id",
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name(INDEX)
                    .table(Invoice::Table)
                    .col(Invoice::WalletId)
                    .col(Invoice::PaymentTime)
                    .and_where(Expr::col(Invoice::MerchantRoute).is_not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(INDEX).table(Invoice::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .drop_column(Invoice::MerchantRoute)
                    .to_owned(),
            )
            .await
    }
}
