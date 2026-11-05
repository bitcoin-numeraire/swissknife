use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20260704_000001_account_table::Account, m20260704_000002_auth_identity_table::AuthIdentity};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    fn use_transaction(&self) -> Option<bool> {
        Some(true)
    }

    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LocalCredential::Table)
                    .col(uuid(LocalCredential::AccountId).primary_key())
                    .col(uuid(LocalCredential::IdentityId).unique_key())
                    .col(text_null(LocalCredential::PasswordHash))
                    .col(boolean(LocalCredential::Enabled).default(true))
                    .col(uuid(LocalCredential::Revision))
                    .col(string_null(LocalCredential::ResetHash).unique_key())
                    .col(timestamp_null(LocalCredential::ResetExpiresAt))
                    .col(timestamp(LocalCredential::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(LocalCredential::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(LocalCredential::Table, LocalCredential::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(LocalCredential::Table, LocalCredential::IdentityId)
                            .to(AuthIdentity::Table, AuthIdentity::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LocalCredential::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LocalCredential {
    Table,
    AccountId,
    IdentityId,
    PasswordHash,
    Enabled,
    Revision,
    ResetHash,
    ResetExpiresAt,
    CreatedAt,
    UpdatedAt,
}
