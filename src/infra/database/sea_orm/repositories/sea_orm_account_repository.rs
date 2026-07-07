use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::user::{Account, AccountRepository, AuthProvider, Permission},
    infra::database::sea_orm::models::{
        account, account_preference, auth_identity,
        prelude::{Account as AccountEntity, AccountPreference, AuthIdentity},
    },
};

#[derive(Clone)]
pub struct SeaOrmAccountRepository {
    db: DatabaseConnection,
}

impl SeaOrmAccountRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AccountRepository for SeaOrmAccountRepository {
    async fn find_by_identity(&self, provider: AuthProvider, subject: &str) -> Result<Option<Account>, DatabaseError> {
        let identity_with_account = AuthIdentity::find()
            .filter(auth_identity::Column::Provider.eq(provider.to_string()))
            .filter(auth_identity::Column::Subject.eq(subject))
            .find_also_related(AccountEntity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        let (identity, account_model) = match identity_with_account {
            Some((identity, Some(account))) => (identity, account),
            Some((_, None)) => {
                return Err(DatabaseError::FindRelated(
                    "auth identity does not reference an account".to_string(),
                ));
            }
            None => return Ok(None),
        };

        let mut account: Account = account_model.into();
        account.identity = Some(identity.into());

        account.preferences = AccountPreference::find_by_id(account.id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(Into::into);

        Ok(Some(account))
    }

    async fn upsert(
        &self,
        provider: AuthProvider,
        subject: &str,
        initial_permissions: &[Permission],
    ) -> Result<Account, DatabaseError> {
        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let account_id = Uuid::new_v4();
        let identity_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        let mut permissions = Vec::new();
        for permission in initial_permissions {
            if !permissions.contains(permission) {
                permissions.push(permission.clone());
            }
        }
        let permissions_json = serde_json::to_value(permissions).map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let account_model = account::ActiveModel {
            id: Set(account_id),
            permissions: Set(permissions_json),
            created_at: Set(now),
            ..Default::default()
        };

        let account_model = account_model
            .insert(&tx)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let preference_model = account_preference::ActiveModel {
            account_id: Set(account_id),
            dashboard_settings: Set(json!({})),
            created_at: Set(now),
            ..Default::default()
        };

        let preference_model = preference_model
            .insert(&tx)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let identity_model = auth_identity::ActiveModel {
            id: Set(identity_id),
            account_id: Set(account_id),
            provider: Set(provider.to_string()),
            subject: Set(subject.to_string()),
            created_at: Set(now),
        };

        let identity_insert = identity_model.insert(&tx).await;

        let identity_model = match identity_insert {
            Ok(identity_model) => identity_model,
            Err(err) => {
                // A concurrent first request can create this identity after the
                // caller's read. The unique index rejects our duplicate insert;
                // return the account that won the race.
                tx.rollback()
                    .await
                    .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

                return self
                    .find_by_identity(provider, subject)
                    .await?
                    .ok_or_else(|| DatabaseError::Insert(err.to_string()));
            }
        };

        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let mut account: Account = account_model.into();
        account.identity = Some(identity_model.into());
        account.preferences = Some(preference_model.into());

        Ok(account)
    }
}
