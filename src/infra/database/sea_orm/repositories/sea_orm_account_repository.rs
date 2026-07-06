use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use super::SeaOrmConnection;

use crate::{
    application::errors::DatabaseError,
    domains::user::{Account, AccountRepository, Permission},
    infra::database::sea_orm::models::{
        account, account_permission, account_preference, auth_identity,
        prelude::{Account as AccountEntity, AccountPermission, AccountPreference, AuthIdentity},
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

    async fn upsert_for_identity(
        &self,
        provider: &str,
        subject: &str,
        initial_permissions: &[Permission],
    ) -> Result<Account, DatabaseError> {
        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        if let Some(account) = find_account_by_identity(&tx, provider, subject).await? {
            insert_permissions(&tx, account.id, initial_permissions).await?;
            let account = find_account_by_identity(&tx, provider, subject)
                .await?
                .ok_or_else(|| DatabaseError::FindOne("account disappeared during upsert".to_string()))?;

            tx.commit()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

            return Ok(account);
        }

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
            tx.rollback()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

            return self
                .find_by_identity(provider, subject)
                .await?
                .ok_or_else(|| DatabaseError::Insert(err.to_string()));
        }

        insert_permissions(&tx, account_id, initial_permissions).await?;

        let account = find_account_by_identity(&tx, provider, subject)
            .await?
            .ok_or_else(|| DatabaseError::Insert("account was not available after provisioning".to_string()))?;

        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        Ok(account)
    }

    async fn upsert_permissions(&self, account_id: Uuid, permissions: &[Permission]) -> Result<(), DatabaseError> {
        insert_permissions(self.db.connection(), account_id, permissions).await
    }
}

async fn find_account_by_identity<C>(db: &C, provider: &str, subject: &str) -> Result<Option<Account>, DatabaseError>
where
    C: ConnectionTrait,
{
    let identity_with_account = AuthIdentity::find()
        .filter(auth_identity::Column::Provider.eq(provider))
        .filter(auth_identity::Column::Subject.eq(subject))
        .find_also_related(AccountEntity)
        .one(db)
        .await
        .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

    match identity_with_account {
        Some((_, Some(account))) => Ok(Some(load_account(db, account).await?)),
        Some((_, None)) => Err(DatabaseError::FindRelated(
            "auth identity does not reference an account".to_string(),
        )),
        None => Ok(None),
    }
}

async fn load_account<C>(db: &C, account_model: account::Model) -> Result<Account, DatabaseError>
where
    C: ConnectionTrait,
{
    let mut account: Account = account_model.clone().into();

    let identities = AuthIdentity::find()
        .filter(auth_identity::Column::AccountId.eq(account_model.id))
        .order_by_asc(auth_identity::Column::Provider)
        .order_by_asc(auth_identity::Column::Subject)
        .all(db)
        .await
        .map_err(|e| DatabaseError::FindMany(e.to_string()))?;
    account.identities = Some(identities.into_iter().map(Into::into).collect());

    account.permissions = Some(find_permissions(db, account_model.id).await?);

    account.preferences = AccountPreference::find_by_id(account_model.id)
        .one(db)
        .await
        .map_err(|e| DatabaseError::FindOne(e.to_string()))?
        .map(Into::into);

    Ok(account)
}

async fn insert_permissions<C>(db: &C, account_id: Uuid, permissions: &[Permission]) -> Result<(), DatabaseError>
where
    C: ConnectionTrait,
{
    if permissions.is_empty() {
        return Ok(());
    }

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
            .exec_without_returning(db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;
    }

    Ok(())
}

async fn find_permissions<C>(db: &C, account_id: Uuid) -> Result<Vec<Permission>, DatabaseError>
where
    C: ConnectionTrait,
{
    AccountPermission::find()
        .filter(account_permission::Column::AccountId.eq(account_id))
        .order_by_asc(account_permission::Column::Permission)
        .all(db)
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
