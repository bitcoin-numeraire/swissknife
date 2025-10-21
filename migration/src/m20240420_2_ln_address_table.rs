use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240420_1_wallet_table::Wallet;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_database_backend();
        let is_postgres = db == DatabaseBackend::Postgres;
        
        let mut table = Table::create()
            .table(LnAddress::Table)
            .if_not_exists()
            .col(uuid(LnAddress::Id).primary_key())
            .col(uuid_uniq(LnAddress::WalletId))
            .col(string_len_uniq(LnAddress::Username, 255))
            .col(boolean(LnAddress::Active).default(true))
            .to_owned();
        
        // Use TIMESTAMPTZ for PostgreSQL, TIMESTAMP for SQLite
        if is_postgres {
            table.col(timestamp_with_time_zone(LnAddress::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_with_time_zone_null(LnAddress::UpdatedAt));
        } else {
            table.col(timestamp(LnAddress::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_null(LnAddress::UpdatedAt));
        }
        
        table.foreign_key(
            ForeignKey::create()
                .name("fk_user")
                .from(LnAddress::Table, LnAddress::WalletId)
                .to(Wallet::Table, Wallet::Id)
                .on_delete(ForeignKeyAction::Cascade),
        );
        
        manager.create_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LnAddress::Table).to_owned())
            .await?;

        manager
            .drop_foreign_key(ForeignKey::drop().name("fk_user").table(LnAddress::Table).to_owned())
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
