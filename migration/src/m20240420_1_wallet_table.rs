use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_database_backend();
        let is_postgres = db == DatabaseBackend::Postgres;
        
        let mut table = Table::create()
            .table(Wallet::Table)
            .if_not_exists()
            .col(uuid(Wallet::Id).primary_key())
            .col(string_len_uniq(Wallet::UserId, 255))
            .to_owned();
        
        // Use TIMESTAMPTZ for PostgreSQL, TIMESTAMP for SQLite
        if is_postgres {
            table.col(timestamp_with_time_zone(Wallet::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_with_time_zone_null(Wallet::UpdatedAt));
        } else {
            table.col(timestamp(Wallet::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_null(Wallet::UpdatedAt));
        }
        
        manager.create_table(table).await
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
    CreatedAt,
    UpdatedAt,
}
