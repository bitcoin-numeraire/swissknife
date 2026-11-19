use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240420_000003_invoice_table::Invoice;

const TOKEN_INDEX: &str = "idx_invoice_lnurl_verify_token";

#[derive(DeriveIden)]
enum Verification {
    LnurlVerifyToken,
    LnurlPreimage,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite supports only one added column per ALTER TABLE.
        for column in [
            uuid_null(Verification::LnurlVerifyToken),
            string_len_null(Verification::LnurlPreimage, 64),
        ] {
            manager
                .alter_table(Table::alter().table(Invoice::Table).add_column(column).to_owned())
                .await?;
        }
        manager
            .create_index(
                Index::create()
                    .name(TOKEN_INDEX)
                    .table(Invoice::Table)
                    .col(Verification::LnurlVerifyToken)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(TOKEN_INDEX).table(Invoice::Table).to_owned())
            .await?;
        for column in [Verification::LnurlVerifyToken, Verification::LnurlPreimage] {
            manager
                .alter_table(Table::alter().table(Invoice::Table).drop_column(column).to_owned())
                .await?;
        }
        Ok(())
    }
}
