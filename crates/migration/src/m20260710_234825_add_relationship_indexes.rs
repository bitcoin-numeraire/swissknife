use sea_orm_migration::prelude::*;

use crate::{
    m20240420_1_wallet_table::Wallet, m20240420_3_invoice_table::Invoice, m20240420_4_payment_table::Payment,
    m20260704_000002_auth_identity_table::AuthIdentity,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_auth_identity_account_id")
                    .table(AuthIdentity::Table)
                    .col(AuthIdentity::AccountId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_wallet_asset_id")
                    .table(Wallet::Table)
                    .col(Wallet::AssetId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invoice_wallet_created_at")
                    .table(Invoice::Table)
                    .col(Invoice::WalletId)
                    .col(Invoice::CreatedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invoice_ln_address_id")
                    .table(Invoice::Table)
                    .col(Invoice::LnAddressId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_invoice_btc_output_id")
                    .table(Invoice::Table)
                    .col(Invoice::BtcOutputId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_payment_wallet_created_at")
                    .table(Payment::Table)
                    .col(Payment::WalletId)
                    .col(Payment::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in [
            "idx_payment_wallet_created_at",
            "idx_invoice_btc_output_id",
            "idx_invoice_ln_address_id",
            "idx_invoice_wallet_created_at",
            "idx_wallet_asset_id",
            "idx_auth_identity_account_id",
        ] {
            manager.drop_index(Index::drop().name(name).to_owned()).await?;
        }

        Ok(())
    }
}
