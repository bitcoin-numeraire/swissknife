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
                    .table(AccountPreference::Table)
                    .if_not_exists()
                    .col(uuid(AccountPreference::AccountId).primary_key())
                    .col(json(AccountPreference::DashboardSettings).default("{}"))
                    .col(timestamp(AccountPreference::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(AccountPreference::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_account_preference_account")
                            .from(AccountPreference::Table, AccountPreference::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AccountPreference::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum AccountPreference {
    Table,
    AccountId,
    DashboardSettings,
    CreatedAt,
    UpdatedAt,
}
