use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20260704_000001_account_table::Account, m20260704_000005_asset_table::Asset};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table
            .table(Wallet::Table)
            .if_not_exists()
            .col(uuid(Wallet::Id).primary_key());

        if manager.get_database_backend() == DatabaseBackend::Sqlite {
            table
                .col(uuid(Wallet::AccountId))
                .col(uuid(Wallet::AssetId))
                .col(text_null(Wallet::Label))
                .col(big_integer(Wallet::AvailableAmount).default(0))
                .col(big_integer(Wallet::ReservedAmount).default(0))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_wallet_account")
                        .from(Wallet::Table, Wallet::AccountId)
                        .to(Account::Table, Account::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_wallet_asset")
                        .from(Wallet::Table, Wallet::AssetId)
                        .to(Asset::Table, Asset::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .check(Expr::col(Wallet::ReservedAmount).gte(0))
                .index(
                    Index::create()
                        .name("idx_wallet_account_asset")
                        .col(Wallet::AccountId)
                        .col(Wallet::AssetId)
                        .unique(),
                )
                .index(
                    Index::create()
                        .name("idx_wallet_account_id")
                        .col(Wallet::AccountId)
                        .col(Wallet::Id)
                        .unique(),
                );
        } else {
            table.col(string_len_null(Wallet::UserId, 255));
        }

        table
            .col(timestamp(Wallet::CreatedAt).default(Expr::current_timestamp()))
            .col(timestamp_null(Wallet::UpdatedAt));

        manager.create_table(table.to_owned()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Wallet::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Wallet {
    Table,
    Id,
    UserId,
    AccountId,
    AssetId,
    Label,
    AvailableAmount,
    ReservedAmount,
    CreatedAt,
    UpdatedAt,
}
