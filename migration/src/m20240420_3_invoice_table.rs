use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_1_wallet_table::Wallet, m20240420_2_ln_address_table::LnAddress};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_database_backend();
        let is_postgres = db == DatabaseBackend::Postgres;
        
        let mut table = Table::create()
            .table(Invoice::Table)
            .if_not_exists()
            .col(uuid(Invoice::Id).primary_key())
            .col(uuid(Invoice::WalletId))
            .col(string_len_null(Invoice::PaymentHash, 255))
            .col(uuid_null(Invoice::LnAddressId))
            .col(string_null(Invoice::Bolt11))
            .col(string_len(Invoice::Ledger, 255))
            .col(string_null(Invoice::PayeePubkey))
            .col(string_null(Invoice::Description))
            .col(string_null(Invoice::DescriptionHash))
            .col(big_integer_null(Invoice::AmountMsat))
            .col(big_integer_null(Invoice::AmountReceivedMsat))
            .col(string_len(Invoice::Currency, 255))
            .col(string_null(Invoice::PaymentSecret))
            .to_owned();
        
        // Use TIMESTAMPTZ for PostgreSQL, TIMESTAMP for SQLite
        if is_postgres {
            table.col(timestamp_with_time_zone(Invoice::Timestamp));
        } else {
            table.col(timestamp(Invoice::Timestamp));
        }
        
        table.col(big_integer_null(Invoice::Expiry));
        table.col(big_integer_null(Invoice::MinFinalCltvExpiryDelta));
        table.col(big_integer_null(Invoice::FeeMsat));
        
        if is_postgres {
            table.col(timestamp_with_time_zone_null(Invoice::PaymentTime));
            table.col(timestamp_with_time_zone(Invoice::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_with_time_zone_null(Invoice::UpdatedAt));
            table.col(timestamp_with_time_zone_null(Invoice::ExpiresAt));
        } else {
            table.col(timestamp_null(Invoice::PaymentTime));
            table.col(timestamp(Invoice::CreatedAt).default(Expr::current_timestamp()));
            table.col(timestamp_null(Invoice::UpdatedAt));
            table.col(timestamp_null(Invoice::ExpiresAt));
        }
        
        table.foreign_key(
                        ForeignKey::create()
                            .name("fk_wallet")
                            .from(Invoice::Table, Invoice::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_ln_address")
                            .from(Invoice::Table, Invoice::LnAddressId)
                            .to(LnAddress::Table, LnAddress::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("UNIQUE_payment_hash")
                            .table(Invoice::Table)
                            .col(Invoice::PaymentHash)
                            .nulls_not_distinct(),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("UNIQUE_bolt11")
                            .table(Invoice::Table)
                            .col(Invoice::Bolt11)
                            .nulls_not_distinct(),
                    );
        
        manager.create_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Invoice::Table).to_owned())
            .await?;

        manager
            .drop_foreign_key(ForeignKey::drop().name("fk_wallet").table(Invoice::Table).to_owned())
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_ln_address")
                    .table(Invoice::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("UNIQUE_payment_hash")
                    .table(Invoice::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(Index::drop().name("UNIQUE_bolt11").table(Invoice::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum Invoice {
    Table,
    Id,
    WalletId,
    PaymentHash,
    LnAddressId,
    Bolt11,
    Ledger,
    PayeePubkey,
    Description,
    DescriptionHash,
    AmountMsat,
    AmountReceivedMsat,
    Currency,
    PaymentSecret,
    Timestamp,
    Expiry,
    MinFinalCltvExpiryDelta,
    FeeMsat,
    PaymentTime,
    CreatedAt,
    UpdatedAt,
    ExpiresAt,
}
