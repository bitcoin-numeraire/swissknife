use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{sea_query::OnConflict, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;
use uuid::Uuid;

use super::SeaOrmConnection;

use crate::{
    application::errors::DatabaseError,
    domains::user::{AccountIdentity, AccountRepository, Permission},
    infra::database::sea_orm::models::{
        account, account_permission, account_preference, auth_identity,
        prelude::{Account, AccountPermission, AccountPreference, AuthIdentity},
    },
};

#[derive(Clone)]
pub struct SeaOrmAccountRepository<C = DatabaseConnection> {
    db: C,
}

impl<C> SeaOrmAccountRepository<C> {
    pub fn new(db: C) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<C> AccountRepository for SeaOrmAccountRepository<C>
where
    C: SeaOrmConnection,
{
    async fn ensure_for_identity(&self, provider: &str, subject: &str) -> Result<AccountIdentity, DatabaseError> {
        if let Some(identity) = self.find_identity(provider, subject).await? {
            return Ok(identity);
        }

        let account_id = deterministic_account_id(provider, subject);
        let now = Utc::now().naive_utc();

        let account_model = account::ActiveModel {
            id: Set(account_id),
            created_at: Set(now),
            ..Default::default()
        };

        let _ = Account::insert(account_model)
            .on_conflict(
                OnConflict::column(account::Column::Id)
                    .do_nothing_on([account::Column::Id])
                    .to_owned(),
            )
            .exec_without_returning(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let preference_model = account_preference::ActiveModel {
            account_id: Set(account_id),
            dashboard_settings: Set(json!({})),
            created_at: Set(now),
            ..Default::default()
        };

        let _ = AccountPreference::insert(preference_model)
            .on_conflict(
                OnConflict::column(account_preference::Column::AccountId)
                    .do_nothing_on([account_preference::Column::AccountId])
                    .to_owned(),
            )
            .exec_without_returning(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let identity_model = auth_identity::ActiveModel {
            id: Set(Uuid::new_v4()),
            account_id: Set(account_id),
            provider: Set(provider.to_string()),
            subject: Set(subject.to_string()),
            created_at: Set(now),
            ..Default::default()
        };

        let _ = AuthIdentity::insert(identity_model)
            .on_conflict(
                OnConflict::columns([auth_identity::Column::Provider, auth_identity::Column::Subject])
                    .do_nothing()
                    .to_owned(),
            )
            .exec_without_returning(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        self.find_identity(provider, subject).await?.ok_or_else(|| {
            DatabaseError::Insert("account identity was not available after idempotent provisioning".to_string())
        })
    }

    async fn grant_permissions(&self, account_id: Uuid, permissions: &[Permission]) -> Result<(), DatabaseError> {
        for permission in permissions {
            let model = account_permission::ActiveModel {
                account_id: Set(account_id),
                permission: Set(permission_storage_value(permission)?),
                created_at: Set(Utc::now().naive_utc()),
            };

            let _ = AccountPermission::insert(model)
                .on_conflict(
                    OnConflict::columns([
                        account_permission::Column::AccountId,
                        account_permission::Column::Permission,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec_without_returning(self.db.connection())
                .await
                .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        }

        Ok(())
    }
}

impl<C> SeaOrmAccountRepository<C>
where
    C: SeaOrmConnection,
{
    async fn find_identity(&self, provider: &str, subject: &str) -> Result<Option<AccountIdentity>, DatabaseError> {
        let identity = AuthIdentity::find()
            .filter(auth_identity::Column::Provider.eq(provider))
            .filter(auth_identity::Column::Subject.eq(subject))
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(identity.map(|model| AccountIdentity {
            account_id: model.account_id,
            provider: model.provider,
            subject: model.subject,
        }))
    }
}

fn deterministic_account_id(provider: &str, subject: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("swissknife:account:{provider}:{subject}").as_bytes(),
    )
}

fn permission_storage_value(permission: &Permission) -> Result<String, DatabaseError> {
    serde_json::to_value(permission)
        .map_err(|e| DatabaseError::Insert(e.to_string()))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| DatabaseError::Insert("permission did not serialize to a string".to_string()))
}

#[cfg(test)]
mod tests {
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

    use super::*;

    async fn sqlite() -> DatabaseConnection {
        let conn = Database::connect("sqlite::memory:").await.expect("connect sqlite");
        Migrator::up(&conn, None).await.expect("run migrations");
        conn
    }

    async fn count(conn: &DatabaseConnection, sql: &str) -> i64 {
        conn.query_one(Statement::from_string(DatabaseBackend::Sqlite, sql.to_string()))
            .await
            .expect("query count")
            .expect("count row")
            .try_get::<i64>("", "count")
            .expect("count value")
    }

    #[tokio::test]
    async fn ensure_for_identity_is_idempotent() {
        let conn = sqlite().await;
        let repo = SeaOrmAccountRepository::new(conn.clone());

        let first = repo.ensure_for_identity("jwt", "alice").await.unwrap();
        let second = repo.ensure_for_identity("jwt", "alice").await.unwrap();

        assert_eq!(first, second);
        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM account").await, 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM auth_identity").await, 1);
        assert_eq!(
            count(&conn, "SELECT COUNT(*) AS count FROM account_preference").await,
            1
        );
    }

    #[tokio::test]
    async fn grant_permissions_is_idempotent() {
        let conn = sqlite().await;
        let repo = SeaOrmAccountRepository::new(conn.clone());
        let identity = repo.ensure_for_identity("jwt", "alice").await.unwrap();

        repo.grant_permissions(identity.account_id, &[Permission::ReadWallet, Permission::ReadWallet])
            .await
            .unwrap();

        assert_eq!(
            count(&conn, "SELECT COUNT(*) AS count FROM account_permission").await,
            1
        );
    }
}
