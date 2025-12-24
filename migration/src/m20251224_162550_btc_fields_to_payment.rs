use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_4_payment_table::Payment, m20251224_162542_btc_transaction_table::BtcTransaction};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add btc_transaction_id column
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(uuid_null(Payment::BtcTxid))
                    .to_owned(),
            )
            .await?;

        // Add destination_address column
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(string_len_null(Payment::DestinationAddress, 255))
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint only for PostgreSQL
        let db_backend = manager.get_database_backend();
        if db_backend == DatabaseBackend::Postgres {
            let fk_payment = TableForeignKey::new()
                .name("fk_payment_btc_transaction")
                .from_tbl(Payment::Table)
                .from_col(Payment::BtcTxid)
                .to_tbl(BtcTransaction::Table)
                .to_col(BtcTransaction::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .to_owned();

            manager
                .alter_table(
                    Table::alter()
                        .table(Payment::Table)
                        .add_foreign_key(&fk_payment)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();

        // Drop foreign key only for PostgreSQL
        if db_backend == DatabaseBackend::Postgres {
            manager
                .alter_table(
                    Table::alter()
                        .table(Payment::Table)
                        .drop_foreign_key(Alias::new("fk_payment_btc_transaction"))
                        .to_owned(),
                )
                .await?;
        }

        // Drop columns
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::BtcTxid)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::DestinationAddress)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
