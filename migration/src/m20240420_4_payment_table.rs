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
            .table(Payment::Table)
            .if_not_exists()
            .col(uuid(Payment::Id).primary_key())
            .col(uuid(Payment::WalletId))
            .col(string_len_null(Payment::LnAddress, 255))
            .col(string_len_null(Payment::PaymentHash, 255).unique_key())
            .col(string_len_null(Payment::PaymentPreimage, 255).unique_key())
            .col(string_null(Payment::Error))
            .col(big_integer(Payment::AmountMsat))
            .col(big_integer_null(Payment::FeeMsat))
            .to_owned();
        
        // Use TIMESTAMPTZ for PostgreSQL, TIMESTAMP for SQLite
        if is_postgres {
            table.col(timestamp_with_time_zone_null(Payment::PaymentTime));
        } else {
            table.col(timestamp_null(Payment::PaymentTime));
        }
        
        table.col(string(Payment::Status));
        table.col(string_len(Payment::Ledger, 255));
        table.col(string_len(Payment::Currency, 255));
        table.col(string_null(Payment::Description));
        table.col(string_null(Payment::Metadata));
        table.col(json_binary_null(Payment::SuccessAction));
        
        if is_postgres {
            table.col(timestamp_with_time_zone(Payment::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_with_time_zone_null(Payment::UpdatedAt));
        } else {
            table.col(timestamp(Payment::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_null(Payment::UpdatedAt));
        }
        
        table.foreign_key(
                        ForeignKey::create()
                            .name("fk_wallet")
                            .from(Payment::Table, Payment::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    );
        
        manager.create_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Payment::Table).to_owned())
            .await?;

        manager
            .drop_foreign_key(ForeignKey::drop().name("fk_wallet").table(Payment::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum Payment {
    Table,
    Id,
    WalletId,
    LnAddress,
    PaymentHash,
    PaymentPreimage,
    Error,
    AmountMsat,
    FeeMsat,
    PaymentTime,
    Status,
    Ledger,
    Currency,
    Description,
    Metadata,
    SuccessAction,
    CreatedAt,
    UpdatedAt,
}
