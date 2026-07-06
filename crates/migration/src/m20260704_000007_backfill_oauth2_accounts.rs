use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;
use uuid::Uuid;

use crate::{
    m20240420_1_wallet_table::Wallet, m20260704_000001_account_table::Account,
    m20260704_000002_auth_identity_table::AuthIdentity,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

const LEGACY_AUTH_PROVIDER: &str = "oauth2";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        backfill_accounts(manager.get_connection()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        execute(
            db,
            backend,
            format!(
                r#"
                DELETE FROM {account}
                WHERE {account_id} IN (
                    SELECT {identity_account_id}
                    FROM {identity}
                    WHERE {provider} = '{legacy_provider}'
                    AND {subject} IN (
                        SELECT {user_id}
                        FROM {wallet}
                    )
                )
                "#,
                account = Account::Table.to_string(),
                account_id = Account::Id.to_string(),
                identity = AuthIdentity::Table.to_string(),
                identity_account_id = AuthIdentity::AccountId.to_string(),
                legacy_provider = sql_literal(LEGACY_AUTH_PROVIDER),
                provider = AuthIdentity::Provider.to_string(),
                subject = AuthIdentity::Subject.to_string(),
                user_id = Wallet::UserId.to_string(),
                wallet = Wallet::Table.to_string(),
            ),
        )
        .await
    }
}

async fn backfill_accounts(db: &dyn ConnectionTrait) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let users = db
        .query_all(Statement::from_string(
            backend,
            format!(
                r#"
                SELECT DISTINCT {user_id}
                FROM {wallet}
                ORDER BY {user_id}
                "#,
                user_id = Wallet::UserId.to_string(),
                wallet = Wallet::Table.to_string(),
            ),
        ))
        .await?;

    for row in users {
        let user_id = row.try_get::<String>("", "user_id")?;
        let account_id = Uuid::new_v4().to_string();
        let auth_identity_id = Uuid::new_v4().to_string();
        let account_id = uuid_literal(backend, &account_id)?;
        let auth_identity_id = uuid_literal(backend, &auth_identity_id)?;

        execute(
            db,
            backend,
            format!(
                r#"
                INSERT INTO account (id, display_name, created_at)
                VALUES ({account_id}, NULL, CURRENT_TIMESTAMP)
                "#
            ),
        )
        .await?;

        execute(
            db,
            backend,
            format!(
                r#"
                INSERT INTO auth_identity (id, account_id, provider, subject, created_at)
                VALUES ({auth_identity_id}, {account_id}, '{provider}', '{subject}', CURRENT_TIMESTAMP)
                "#,
                provider = sql_literal(LEGACY_AUTH_PROVIDER),
                subject = sql_literal(&user_id),
            ),
        )
        .await?;

        execute(
            db,
            backend,
            format!(
                r#"
                INSERT INTO account_preference (account_id, dashboard_settings, created_at)
                VALUES ({account_id}, '{{}}', CURRENT_TIMESTAMP)
                "#
            ),
        )
        .await?;
    }

    Ok(())
}

async fn execute(db: &dyn ConnectionTrait, backend: DatabaseBackend, sql: String) -> Result<(), DbErr> {
    db.execute(Statement::from_string(backend, sql)).await?;
    Ok(())
}

fn sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn uuid_literal(backend: DatabaseBackend, value: &str) -> Result<String, DbErr> {
    let uuid =
        Uuid::parse_str(value).map_err(|err| DbErr::Migration(format!("invalid UUID literal {value}: {err}")))?;

    Ok(match backend {
        DatabaseBackend::Sqlite => format!("X'{}'", uuid.simple()),
        _ => format!("'{uuid}'"),
    })
}
