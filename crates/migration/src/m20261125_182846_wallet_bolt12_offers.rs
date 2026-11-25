use sea_orm_migration::{prelude::*, sea_orm::TransactionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&txn);
        use WalletOffer::{Table as Offers, *};
        schema
            .create_table(
                sea_orm_migration::prelude::Table::create()
                    .table(Offers)
                    .col(ColumnDef::new(Id).uuid().primary_key())
                    .col(ColumnDef::new(WalletId).uuid().not_null())
                    .col(ColumnDef::new(RequestKey).uuid().not_null())
                    .col(ColumnDef::new(Backend).string().not_null())
                    .col(ColumnDef::new(Description).text().not_null())
                    .col(ColumnDef::new(AmountMsat).big_integer())
                    .col(ColumnDef::new(SingleUse).boolean().not_null())
                    .col(ColumnDef::new(State).string().not_null())
                    .col(ColumnDef::new(ProviderId).string().unique_key())
                    .col(ColumnDef::new(Bolt12).text())
                    .col(
                        ColumnDef::new(CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Offers, WalletId)
                            .to(Alias::new("wallet"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().unique().col(WalletId).col(RequestKey))
                    .to_owned(),
            )
            .await?;
        schema
            .create_table(
                sea_orm_migration::prelude::Table::create()
                    .table(Alias::new("offer_cursor"))
                    .col(ColumnDef::new(Backend).string().primary_key())
                    .col(ColumnDef::new(Alias::new("last_index")).big_integer().not_null())
                    .to_owned(),
            )
            .await?;
        schema
            .create_table(
                sea_orm_migration::prelude::Table::create()
                    .table(Alias::new("offer_receipt"))
                    .col(ColumnDef::new(Alias::new("payment_hash")).string().primary_key())
                    .col(ColumnDef::new(Alias::new("offer_id")).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("offer_receipt"), Alias::new("offer_id"))
                            .to(Offers, Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        for column in [
            ColumnDef::new(Alias::new("offer_id"))
                .uuid()
                .extra("REFERENCES wallet_offer(id)".to_owned())
                .to_owned(),
            ColumnDef::new(Alias::new("bolt12")).text().to_owned(),
            ColumnDef::new(Alias::new("payer_note")).text().to_owned(),
        ] {
            schema
                .alter_table(
                    sea_orm_migration::prelude::Table::alter()
                        .table(Alias::new("invoice"))
                        .add_column(column)
                        .to_owned(),
                )
                .await?;
        }
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&txn);
        for column in ["payer_note", "bolt12", "offer_id"] {
            schema
                .alter_table(
                    Table::alter()
                        .table(Alias::new("invoice"))
                        .drop_column(Alias::new(column))
                        .to_owned(),
                )
                .await?;
        }
        for table in ["offer_receipt", "offer_cursor", "wallet_offer"] {
            schema
                .drop_table(Table::drop().table(Alias::new(table)).to_owned())
                .await?;
        }
        txn.commit().await
    }
}

#[derive(DeriveIden)]
enum WalletOffer {
    Table,
    Id,
    WalletId,
    RequestKey,
    Backend,
    Description,
    AmountMsat,
    SingleUse,
    State,
    ProviderId,
    Bolt12,
    CreatedAt,
}
