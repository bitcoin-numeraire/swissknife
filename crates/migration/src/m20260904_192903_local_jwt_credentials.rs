use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::{prelude::*, schema::*};
use uuid::Uuid;

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

        let db = manager.get_connection();
        let backend = manager.get_database_backend();
        let legacy = db
            .query_one_raw(Statement::from_string(
                backend,
                "SELECT value FROM config WHERE key = 'password_hash'".to_string(),
            ))
            .await?;
        if let Some(legacy) = legacy {
            let value: sea_orm::JsonValue = legacy.try_get("", "value")?;
            let password_hash = value
                .as_str()
                .ok_or_else(|| DbErr::Migration("Invalid legacy password hash".into()))?;
            let identity = db
                .query_one_raw(Statement::from_string(
                    backend,
                    "SELECT id, account_id FROM auth_identity WHERE provider = 'jwt' AND subject = 'admin'".to_string(),
                ))
                .await?
                .ok_or_else(|| {
                    DbErr::Migration("Legacy credentials have no jwt/admin identity; reconcile before upgrading".into())
                })?;
            let account_id: Uuid = identity.try_get("", "account_id")?;
            let identity_id: Uuid = identity.try_get("", "id")?;
            let insert = Query::insert()
                .into_table(LocalCredential::Table)
                .columns([
                    LocalCredential::AccountId,
                    LocalCredential::IdentityId,
                    LocalCredential::PasswordHash,
                    LocalCredential::Revision,
                ])
                .values_panic([
                    account_id.into(),
                    identity_id.into(),
                    password_hash.into(),
                    Uuid::new_v4().into(),
                ])
                .to_owned();
            db.execute_raw(backend.build(&insert)).await?;
            db.execute_raw(Statement::from_string(backend,
                "INSERT INTO config (key, value) VALUES ('local_auth_initialized', 'true') ON CONFLICT (key) DO NOTHING".to_string())).await?;
            db.execute_raw(Statement::from_string(
                backend,
                "DELETE FROM config WHERE key = 'password_hash'".to_string(),
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Local credentials cannot be downgraded to a shared password; restore the pre-upgrade backup".into(),
        ))
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
