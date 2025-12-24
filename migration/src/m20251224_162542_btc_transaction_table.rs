use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BtcTransaction::Table)
                    .if_not_exists()
                    .col(uuid(BtcTransaction::Id).primary_key())
                    .col(string_len(BtcTransaction::Txid, 255).unique_key())
                    .col(big_integer(BtcTransaction::AmountSat))
                    .col(big_integer_null(BtcTransaction::FeeSat))
                    .col(integer_null(BtcTransaction::BlockHeight))
                    .col(timestamp_null(BtcTransaction::Timestamp))
                    .col(string_len(BtcTransaction::Currency, 255))
                    .col(timestamp(BtcTransaction::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(BtcTransaction::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop table
        manager
            .drop_table(Table::drop().table(BtcTransaction::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum BtcTransaction {
    Table,
    Id,
    Txid,
    AmountSat,
    FeeSat,
    BlockHeight,
    Timestamp,
    Currency,
    CreatedAt,
    UpdatedAt,
}
