use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use super::SeaOrmConnection;

use crate::{
    application::errors::DatabaseError,
    domains::user::{Account, AccountRepository, Permission},
    infra::database::sea_orm::models::{
        account, account_permission, account_preference, auth_identity,
        prelude::{Account as AccountEntity, AccountPermission, AuthIdentity},
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

impl<C> SeaOrmAccountRepository<C>
where
    C: SeaOrmConnection,
{
    async fn find_by_identity_internal(&self, provider: &str, subject: &str) -> Result<Option<Account>, DatabaseError> {
        find_account_by_identity(self.db.connection(), provider, subject).await
    }
}

#[async_trait]
impl AccountRepository for SeaOrmAccountRepository<DatabaseConnection> {
    async fn find_by_identity(&self, provider: &str, subject: &str) -> Result<Option<Account>, DatabaseError> {
        self.find_by_identity_internal(provider, subject).await
    }

    async fn upsert_for_identity(&self, provider: &str, subject: &str) -> Result<Account, DatabaseError> {
        if let Some(account) = self.find_by_identity(provider, subject).await? {
            return Ok(account);
        }

        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        let account_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();

        let account_model = account::ActiveModel {
            id: Set(account_id),
            created_at: Set(now),
            ..Default::default()
        };

        let account = account_model
            .insert(&tx)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let preference_model = account_preference::ActiveModel {
            account_id: Set(account_id),
            dashboard_settings: Set(json!({})),
            created_at: Set(now),
            ..Default::default()
        };

        preference_model
            .insert(&tx)
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

        let identity_insert = identity_model.insert(&tx).await;

        if let Err(err) = identity_insert {
            tx.rollback()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

            return self
                .find_by_identity(provider, subject)
                .await?
                .ok_or_else(|| DatabaseError::Insert(err.to_string()));
        }

        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(account.into())
    }

    async fn upsert_permissions(&self, account_id: Uuid, permissions: &[Permission]) -> Result<(), DatabaseError> {
        let now = Utc::now().naive_utc();

        for permission in permissions {
            let model = account_permission::ActiveModel {
                account_id: Set(account_id),
                permission: Set(permission.to_string()),
                created_at: Set(now),
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

    async fn find_permissions(&self, account_id: Uuid) -> Result<Vec<Permission>, DatabaseError> {
        AccountPermission::find()
            .filter(account_permission::Column::AccountId.eq(account_id))
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?
            .into_iter()
            .map(|model| {
                model
                    .permission
                    .parse()
                    .map_err(|e: strum::ParseError| DatabaseError::FindMany(e.to_string()))
            })
            .collect()
    }
}

async fn find_account_by_identity<C>(db: &C, provider: &str, subject: &str) -> Result<Option<Account>, DatabaseError>
where
    C: ConnectionTrait,
{
    let account = AuthIdentity::find()
        .filter(auth_identity::Column::Provider.eq(provider))
        .filter(auth_identity::Column::Subject.eq(subject))
        .find_also_related(AccountEntity)
        .one(db)
        .await
        .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

    match account {
        Some((_, Some(account))) => Ok(Some(account.into())),
        Some((_, None)) => Err(DatabaseError::FindRelated(
            "auth identity does not reference an account".to_string(),
        )),
        None => Ok(None),
    }
}
