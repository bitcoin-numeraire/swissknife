use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240420_1_wallet_table::Wallet;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        add_wallet_column(manager, uuid_null(Wallet::AccountId)).await?;
        add_wallet_column(manager, uuid_null(Wallet::AssetId)).await?;
        add_wallet_column(manager, text_null(Wallet::Label)).await?;

        let mut available_amount = big_integer(Wallet::AvailableAmount);
        available_amount.default(0);
        add_wallet_column(manager, available_amount).await?;

        let mut reserved_amount = big_integer(Wallet::ReservedAmount);
        reserved_amount.default(0);
        add_wallet_column(manager, reserved_amount).await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wallet_account_asset")
                    .table(Wallet::Table)
                    .col(Wallet::AccountId)
                    .col(Wallet::AssetId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wallet_account_id")
                    .table(Wallet::Table)
                    .col(Wallet::AccountId)
                    .col(Wallet::Id)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        manager
            .drop_index(
                Index::drop()
                    .name("idx_wallet_account_id")
                    .table(Wallet::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_wallet_account_asset")
                    .table(Wallet::Table)
                    .to_owned(),
            )
            .await?;

        drop_wallet_column(manager, Wallet::ReservedAmount).await?;
        drop_wallet_column(manager, Wallet::AvailableAmount).await?;
        drop_wallet_column(manager, Wallet::Label).await?;
        drop_wallet_column(manager, Wallet::AssetId).await?;
        drop_wallet_column(manager, Wallet::AccountId).await
    }
}

async fn add_wallet_column(manager: &SchemaManager<'_>, column: ColumnDef) -> Result<(), DbErr> {
    manager
        .alter_table(Table::alter().table(Wallet::Table).add_column(column).to_owned())
        .await
}

async fn drop_wallet_column(manager: &SchemaManager<'_>, column: Wallet) -> Result<(), DbErr> {
    manager
        .alter_table(Table::alter().table(Wallet::Table).drop_column(column).to_owned())
        .await
}
