use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::user::{Account, AccountRepository, AuthProvider, Permission},
    infra::database::sea_orm::models::{
        account, account_permission, account_preference, auth_identity,
        prelude::{Account as AccountEntity, AccountPermission, AccountPreference, AuthIdentity},
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

        account.permissions = Some(
            AccountPermission::find()
                .filter(account_permission::Column::AccountId.eq(account.id))
                .order_by_asc(account_permission::Column::Permission)
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
                .collect::<Result<Vec<_>, _>>()?,
        );

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

        account_model
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
            // A concurrent first request can create this identity after the
            // pre-read. The unique index rejects our duplicate insert; return
            // the account that won the race.
            tx.rollback()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

            return self
                .find_by_identity(provider, subject)
                .await?
                .ok_or_else(|| DatabaseError::Insert(err.to_string()));
        }

        for permission in initial_permissions {
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
                .exec_without_returning(&tx)
                .await
                .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        self.find_by_identity(provider, subject)
            .await?
            .ok_or_else(|| DatabaseError::Insert("account was not available after provisioning".to_string()))
    }
}
