use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260704_000001_account_table::Account;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuthIdentity::Table)
                    .if_not_exists()
                    .col(uuid(AuthIdentity::Id).primary_key())
                    .col(uuid(AuthIdentity::AccountId))
                    .col(string_len(AuthIdentity::Provider, 255))
                    .col(string_len(AuthIdentity::Subject, 255))
                    .col(timestamp(AuthIdentity::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(AuthIdentity::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_auth_identity_account")
                            .from(AuthIdentity::Table, AuthIdentity::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_auth_identity_provider_subject")
                    .table(AuthIdentity::Table)
                    .col(AuthIdentity::Provider)
                    .col(AuthIdentity::Subject)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_auth_identity_provider_subject")
                    .table(AuthIdentity::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(AuthIdentity::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum AuthIdentity {
    Table,
    Id,
    AccountId,
    Provider,
    Subject,
    CreatedAt,
    UpdatedAt,
}
