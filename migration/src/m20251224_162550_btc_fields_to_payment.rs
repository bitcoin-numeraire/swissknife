use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_4_payment_table::Payment, m20251224_162542_btc_output_table::BtcOutput};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add btc_output_id column
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(uuid_null(Payment::BtcOutputId))
                    .to_owned(),
            )
            .await?;

        // Add destination_address column
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(string_len_null(Payment::BtcAddress, 255))
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint only for PostgreSQL
        let db_backend = manager.get_database_backend();
        if db_backend == DatabaseBackend::Postgres {
            let fk_payment = TableForeignKey::new()
                .name("fk_payment_btc_output")
                .from_tbl(Payment::Table)
                .from_col(Payment::BtcOutputId)
                .to_tbl(BtcOutput::Table)
                .to_col(BtcOutput::Id)
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
                        .drop_foreign_key(Alias::new("fk_payment_btc_output"))
                        .to_owned(),
                )
                .await?;
        }

        // Drop columns
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::BtcOutputId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::BtcAddress)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
