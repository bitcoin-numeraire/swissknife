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
                    .table(LnAddress::Table)
                    .if_not_exists()
                    .col(uuid(LnAddress::Id).primary_key())
                    .col(uuid_uniq(LnAddress::WalletId))
                    .col(string_len_uniq(LnAddress::Username, 255))
                    .col(boolean(LnAddress::Active).default(true))
                    .col(timestamp(LnAddress::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(LnAddress::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user")
                            .from(LnAddress::Table, LnAddress::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LnAddress::Table).to_owned())
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_user")
                    .table(LnAddress::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum LnAddress {
    Table,
    Id,
    WalletId,
    Username,
    Active,
    CreatedAt,
    UpdatedAt,
    AllowsNostr,
    NostrPubkey,
}
