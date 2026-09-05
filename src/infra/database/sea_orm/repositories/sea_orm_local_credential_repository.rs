use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::account::{LocalCredential, LocalCredentialRepository, Permission, LOCAL_AUTH_INITIALIZED_KEY},
    infra::database::sea_orm::models::{
        account, account_preference, auth_identity, config, local_credential,
        prelude::LocalCredential as LocalCredentialEntity,
    },
};

pub struct SeaOrmLocalCredentialRepository {
    db: DatabaseConnection,
}

impl SeaOrmLocalCredentialRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn hydrate(
        model: local_credential::Model,
        identity: Option<auth_identity::Model>,
    ) -> Result<LocalCredential, ApplicationError> {
        let identity = identity
            .filter(|i| i.provider == "jwt" && i.account_id == model.account_id)
            .ok_or_else(|| DataError::Inconsistency("Local credential identity mismatch".into()))?;
        Ok(LocalCredential {
            account_id: model.account_id,
            identity_id: model.identity_id,
            subject: identity.subject,
            password_hash: model.password_hash,
            enabled: model.enabled,
            revision: model.revision,
            reset_hash: model.reset_hash,
            reset_expires_at: model.reset_expires_at.map(|d| d.and_utc()),
        })
    }
}

#[async_trait]
impl LocalCredentialRepository for SeaOrmLocalCredentialRepository {
    async fn find(&self, account_id: Uuid) -> Result<Option<LocalCredential>, ApplicationError> {
        LocalCredentialEntity::find_by_id(account_id)
            .find_also_related(auth_identity::Entity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(|(credential, identity)| Self::hydrate(credential, identity))
            .transpose()
    }

    async fn find_by_subject(&self, subject: &str) -> Result<Option<LocalCredential>, ApplicationError> {
        LocalCredentialEntity::find()
            .find_also_related(auth_identity::Entity)
            .filter(auth_identity::Column::Subject.eq(subject))
            .filter(auth_identity::Column::Provider.eq("jwt"))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(|(credential, identity)| Self::hydrate(credential, identity))
            .transpose()
    }

    async fn find_by_reset_hash(&self, hash: &str) -> Result<Option<LocalCredential>, ApplicationError> {
        LocalCredentialEntity::find()
            .filter(local_credential::Column::ResetHash.eq(hash))
            .find_also_related(auth_identity::Entity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(|(credential, identity)| Self::hydrate(credential, identity))
            .transpose()
    }

    async fn bootstrap(&self, password_hash: String, permissions: Vec<Permission>) -> Result<Uuid, ApplicationError> {
        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        // The first statement claims a durable marker; no read-then-write race on SQLite.
        let claimed = config::Entity::insert(config::ActiveModel {
            key: Set(LOCAL_AUTH_INITIALIZED_KEY.into()),
            value: Set(Some(json!(true))),
        })
        .on_conflict(OnConflict::column(config::Column::Key).do_nothing().to_owned())
        .exec_without_returning(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        if claimed != 1 {
            return Err(DataError::Conflict("Owner setup is already complete".into()).into());
        }
        let account_id = Uuid::new_v4();
        let identity_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        account::ActiveModel {
            id: Set(account_id),
            display_name: Set(None),
            permissions: Set(json!(permissions)),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        account_preference::ActiveModel {
            account_id: Set(account_id),
            dashboard_settings: Set(json!({})),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        auth_identity::ActiveModel {
            id: Set(identity_id),
            account_id: Set(account_id),
            provider: Set("jwt".into()),
            subject: Set("admin".into()),
            created_at: Set(now),
        }
        .insert(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        local_credential::ActiveModel {
            account_id: Set(account_id),
            identity_id: Set(identity_id),
            password_hash: Set(Some(password_hash)),
            enabled: Set(true),
            revision: Set(Uuid::new_v4()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        Ok(account_id)
    }

    async fn create(
        &self,
        account_id: Uuid,
        subject: String,
        reset_hash: String,
        reset_expires_at: DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        // Claim the account row before reads, serializing credential attachment against deletion.
        let exists = account::Entity::update_many()
            .col_expr(
                account::Column::UpdatedAt,
                sea_orm::sea_query::Expr::current_timestamp(),
            )
            .filter(account::Column::Id.eq(account_id))
            .exec(&tx)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        if exists.rows_affected == 0 {
            return Err(DataError::NotFound("Account not found".into()).into());
        }
        if LocalCredentialEntity::find_by_id(account_id)
            .one(&tx)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .is_some()
        {
            return Err(DataError::Conflict("Account already has local credentials".into()).into());
        }
        let existing = auth_identity::Entity::find()
            .filter(auth_identity::Column::AccountId.eq(account_id))
            .one(&tx)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;
        if existing.is_some() {
            return Err(DataError::Conflict("Account already has a login identity".into()).into());
        }
        let now = Utc::now().naive_utc();
        let identity_id = Uuid::new_v4();
        let inserted = auth_identity::Entity::insert(auth_identity::ActiveModel {
            id: Set(identity_id),
            account_id: Set(account_id),
            provider: Set("jwt".into()),
            subject: Set(subject),
            created_at: Set(now),
        })
        .on_conflict(
            OnConflict::columns([auth_identity::Column::Provider, auth_identity::Column::Subject])
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        if inserted != 1 {
            return Err(DataError::Conflict("Username is already in use".into()).into());
        }
        local_credential::ActiveModel {
            account_id: Set(account_id),
            identity_id: Set(identity_id),
            password_hash: Set(None),
            enabled: Set(true),
            revision: Set(Uuid::new_v4()),
            reset_hash: Set(Some(reset_hash)),
            reset_expires_at: Set(Some(reset_expires_at.naive_utc())),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        Ok(())
    }

    async fn replace(&self, credential: LocalCredential, expected_revision: Uuid) -> Result<(), ApplicationError> {
        let affected = LocalCredentialEntity::update_many()
            .set(local_credential::ActiveModel {
                password_hash: Set(credential.password_hash),
                enabled: Set(credential.enabled),
                revision: Set(credential.revision),
                reset_hash: Set(credential.reset_hash),
                reset_expires_at: Set(credential.reset_expires_at.map(|d| d.naive_utc())),
                updated_at: Set(Some(Utc::now().naive_utc())),
                ..Default::default()
            })
            .filter(local_credential::Column::AccountId.eq(credential.account_id))
            .filter(local_credential::Column::Revision.eq(expected_revision))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        if affected.rows_affected != 1 {
            return Err(DataError::Conflict("Login changed; retry the operation".into()).into());
        }
        Ok(())
    }
}
