use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240420_000003_invoice_table::Invoice;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum WalletCredit {
    Table,
    RequestId,
    InvoiceId,
    ActorAccountId,
    ApiKeyId,
    TargetAccountId,
    WalletId,
    AssetId,
    AmountUnits,
    Reason,
    Purpose,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Activity {
    CreditPurpose,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .add_column(string_null(Activity::CreditPurpose))
                    .to_owned(),
            )
            .await?;
        // Keep the immutable receipt when the target, actor or API key is removed.
        manager
            .create_table(
                Table::create()
                    .table(WalletCredit::Table)
                    .col(uuid(WalletCredit::RequestId).primary_key())
                    .col(uuid(WalletCredit::InvoiceId).unique_key())
                    .col(uuid(WalletCredit::ActorAccountId))
                    .col(uuid_null(WalletCredit::ApiKeyId))
                    .col(uuid(WalletCredit::TargetAccountId))
                    .col(uuid(WalletCredit::WalletId))
                    .col(uuid(WalletCredit::AssetId))
                    .col(big_integer(WalletCredit::AmountUnits))
                    .col(string_len(WalletCredit::Reason, 240))
                    .col(string_len(WalletCredit::Purpose, 16))
                    .col(date_time(WalletCredit::CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_wallet_credit_opening")
                    .table(WalletCredit::Table)
                    .col(WalletCredit::WalletId)
                    .unique()
                    .and_where(Expr::col(WalletCredit::Purpose).eq("opening"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Older binaries cannot interpret Adjustment invoices. Refuse to erase
        // live receipts or leave their invoice rows unreadable during a rollback.
        let query = Query::select()
            .column(WalletCredit::RequestId)
            .from(WalletCredit::Table)
            .limit(1)
            .to_owned();
        let conn = manager.get_connection();
        if conn.query_one(&query).await?.is_some() {
            return Err(DbErr::Custom(
                "Wallet credit history must be preserved before downgrading".into(),
            ));
        }
        manager
            .drop_table(Table::drop().table(WalletCredit::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .drop_column(Activity::CreditPurpose)
                    .to_owned(),
            )
            .await
    }
}
