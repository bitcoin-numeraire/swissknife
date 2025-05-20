use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::{
    m20240420_135900_wallet_table::Wallet, m20240420_135901_ln_address_table::LnAddress,
    m20240420_135902_invoice_table::Invoice, m20240420_135903_payment_table::Payment,
    m20241009_135905_api_key_table::ApiKey,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

// This migration is specific to Postgres where the timestamp column were defined with timezone
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();

        if db_backend == DatabaseBackend::Postgres {
            manager
                .alter_table(
                    Table::alter()
                        .table(Wallet::Table)
                        .modify_column(timestamp(Wallet::CreatedAt).default(Expr::current_timestamp()))
                        .modify_column(timestamp_null(Wallet::UpdatedAt))
                        .to_owned(),
                )
                .await?;

            manager
                .alter_table(
                    Table::alter()
                        .table(LnAddress::Table)
                        .modify_column(timestamp(LnAddress::CreatedAt).default(Expr::current_timestamp()))
                        .modify_column(timestamp_null(LnAddress::UpdatedAt))
                        .to_owned(),
                )
                .await?;

            manager
                .alter_table(
                    Table::alter()
                        .table(Invoice::Table)
                        .modify_column(timestamp(Invoice::Timestamp))
                        .modify_column(timestamp_null(Invoice::PaymentTime))
                        .modify_column(timestamp(Invoice::CreatedAt).default(Expr::current_timestamp()))
                        .modify_column(timestamp_null(Invoice::UpdatedAt))
                        .modify_column(timestamp_null(Invoice::ExpiresAt))
                        .to_owned(),
                )
                .await?;

            manager
                .alter_table(
                    Table::alter()
                        .table(Payment::Table)
                        .modify_column(timestamp_null(Payment::PaymentTime))
                        .modify_column(timestamp(Payment::CreatedAt).default(Expr::current_timestamp()))
                        .modify_column(timestamp_null(Payment::UpdatedAt))
                        .to_owned(),
                )
                .await?;

            manager
                .alter_table(
                    Table::alter()
                        .table(ApiKey::Table)
                        .modify_column(timestamp(ApiKey::CreatedAt).default(Expr::current_timestamp()))
                        .modify_column(timestamp_null(ApiKey::ExpiresAt))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
