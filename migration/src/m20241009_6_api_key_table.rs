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
            .table(ApiKey::Table)
            .if_not_exists()
            .col(uuid(ApiKey::Id).primary_key())
            .col(string_len(ApiKey::UserId, 255))
            .col(string_len(ApiKey::Name, 255))
            .col(binary_len_uniq(ApiKey::KeyHash, 32))
            .col(json(ApiKey::Permissions))
            .col(text_null(ApiKey::Description))
            .to_owned();
        
        // Use TIMESTAMPTZ for PostgreSQL, TIMESTAMP for SQLite
        if is_postgres {
            table.col(timestamp_with_time_zone(ApiKey::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_with_time_zone_null(ApiKey::ExpiresAt));
        } else {
            table.col(timestamp(ApiKey::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_null(ApiKey::ExpiresAt));
        }
        
        table.foreign_key(
            ForeignKey::create()
                .from(ApiKey::Table, ApiKey::UserId)
                .to(Wallet::Table, Wallet::UserId)
                .on_delete(ForeignKeyAction::Cascade),
        );
        
        manager.create_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKey::Table).to_owned())
            .await?;

        manager
            .drop_foreign_key(ForeignKey::drop().table(ApiKey::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum ApiKey {
    Table,
    Id,
    UserId,
    Name,
    KeyHash,
    Permissions,
    Description,
    CreatedAt,
    ExpiresAt,
}
