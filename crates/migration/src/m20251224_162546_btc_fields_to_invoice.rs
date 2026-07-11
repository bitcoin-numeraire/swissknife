use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20240420_3_invoice_table::Invoice, m20251224_162542_btc_output_table::BtcOutput};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .add_column(uuid_null(Invoice::BtcOutputId))
                    .to_owned(),
            )
            .await?;

        let fk_invoice = TableForeignKey::new()
            .name("fk_invoice_btc_output")
            .from_tbl(Invoice::Table)
            .from_col(Invoice::BtcOutputId)
            .to_tbl(BtcOutput::Table)
            .to_col(BtcOutput::Id)
            .on_delete(ForeignKeyAction::SetNull)
            .to_owned();

        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .add_foreign_key(&fk_invoice)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .drop_foreign_key(Alias::new("fk_invoice_btc_output"))
                    .to_owned(),
            )
            .await?;

        // Drop columns
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .drop_column(Invoice::BtcOutputId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
