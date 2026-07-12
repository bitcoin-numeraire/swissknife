use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_1_wallet_table::Wallet, m20260704_000001_account_table::Account};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table
            .table(LnAddress::Table)
            .if_not_exists()
            .col(uuid(LnAddress::Id).primary_key());

        if manager.get_database_backend() == DatabaseBackend::Sqlite {
            table
                .col(uuid(LnAddress::AccountId))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_ln_address_account")
                        .from(LnAddress::Table, LnAddress::AccountId)
                        .to(Account::Table, Account::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_ln_address_wallet")
                        .from(LnAddress::Table, LnAddress::AccountId)
                        .from(LnAddress::Table, LnAddress::WalletId)
                        .to(Wallet::Table, Wallet::AccountId)
                        .to(Wallet::Table, Wallet::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .index(
                    Index::create()
                        .name("idx_ln_address_account")
                        .col(LnAddress::AccountId)
                        .unique(),
                );
        }

        table
            .col(uuid_uniq(LnAddress::WalletId))
            .col(string_len_uniq(LnAddress::Username, 255))
            .col(boolean(LnAddress::Active).default(true))
            .col(timestamp(LnAddress::CreatedAt).default(Expr::current_timestamp()))
            .col(timestamp_null(LnAddress::UpdatedAt));

        if manager.get_database_backend() != DatabaseBackend::Sqlite {
            table.foreign_key(
                ForeignKey::create()
                    .name("fk_ln_address_wallet")
                    .from(LnAddress::Table, LnAddress::WalletId)
                    .to(Wallet::Table, Wallet::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            );
        }

        manager.create_table(table.to_owned()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LnAddress::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum LnAddress {
    Table,
    Id,
    WalletId,
    AccountId,
    Username,
    Active,
    CreatedAt,
    UpdatedAt,
    AllowsNostr,
    NostrPubkey,
}
