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
                    .table(AccountPermission::Table)
                    .if_not_exists()
                    .col(uuid(AccountPermission::AccountId))
                    .col(string_len(AccountPermission::Permission, 255))
                    .col(timestamp(AccountPermission::CreatedAt).default(Expr::current_timestamp()))
                    .primary_key(
                        Index::create()
                            .name("pk_account_permission")
                            .col(AccountPermission::AccountId)
                            .col(AccountPermission::Permission),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_account_permission_account")
                            .from(AccountPermission::Table, AccountPermission::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AccountPermission::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum AccountPermission {
    Table,
    AccountId,
    Permission,
    CreatedAt,
}
