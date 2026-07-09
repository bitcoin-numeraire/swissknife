use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::user::{Account, AccountPreferences, AccountRepository, AuthProvider, Permission},
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

    async fn find_preferences(&self, id: Uuid) -> Result<Option<AccountPreferences>, DatabaseError> {
        let preference = AccountPreference::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(preference.map(Into::into))
    }
}

#[async_trait]
impl AccountRepository for SeaOrmAccountRepository {
    async fn find(&self, id: Uuid) -> Result<Option<Account>, DatabaseError> {
        let Some((account_model, preference_model)) = AccountEntity::find_by_id(id)
            .find_also_related(AccountPreference)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
        else {
            return Ok(None);
        };

        let identity = AuthIdentity::find()
            .filter(auth_identity::Column::AccountId.eq(id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .ok_or_else(|| DatabaseError::FindRelated("account does not reference an auth identity".to_string()))?;

        let preference_model = preference_model
            .ok_or_else(|| DatabaseError::FindRelated("account does not reference preferences".to_string()))?;

        let mut account: Account = account_model.into();
        account.identity = Some(identity.into());
        account.preferences = Some(preference_model.into());

        Ok(Some(account))
    }

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
        account.preferences = self.find_preferences(account.id).await?;

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

    async fn update_preferences(
        &self,
        id: Uuid,
        dashboard_settings: Value,
    ) -> Result<Option<AccountPreferences>, DatabaseError> {
        let now = Utc::now().naive_utc();

        let Some(existing) = AccountPreference::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
        else {
            return Ok(None);
        };

        let mut preference: account_preference::ActiveModel = existing.into();
        preference.dashboard_settings = Set(dashboard_settings);
        preference.updated_at = Set(Some(now));

        let preference = preference
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(Some(preference.into()))
    }
}
