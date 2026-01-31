use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BtcOutput::Table)
                    .if_not_exists()
                    .col(uuid(BtcOutput::Id).primary_key())
                    .col(string_len(BtcOutput::Outpoint, 255).unique_key())
                    .col(string_len(BtcOutput::Txid, 255))
                    .col(unsigned(BtcOutput::OutputIndex))
                    .col(string_len(BtcOutput::Address, 255))
                    .col(big_integer(BtcOutput::AmountSat))
                    .col(string(BtcOutput::Status))
                    .col(unsigned_null(BtcOutput::BlockHeight))
                    .col(string_len(BtcOutput::Network, 255))
                    .col(timestamp(BtcOutput::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(BtcOutput::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_btc_output_txid_output_index")
                    .table(BtcOutput::Table)
                    .col(BtcOutput::Txid)
                    .col(BtcOutput::OutputIndex)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_btc_output_txid_output_index")
                    .table(BtcOutput::Table)
                    .to_owned(),
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(BtcOutput::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum BtcOutput {
    Table,
    Id,
    Outpoint,
    Txid,
    OutputIndex,
    Address,
    AmountSat,
    Status,
    BlockHeight,
    Network,
    CreatedAt,
    UpdatedAt,
}
