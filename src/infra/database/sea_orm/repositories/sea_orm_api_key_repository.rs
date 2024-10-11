use crate::{
    application::errors::DatabaseError,
    domains::user::{ApiKey, ApiKeyFilter, ApiKeyRepository},
    infra::database::sea_orm::models::api_key::{ActiveModel, Column, Entity},
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct SeaOrmApiKeyRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmApiKeyRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ApiKeyRepository for SeaOrmApiKeyRepository {
    async fn find(&self, id: Uuid) -> Result<Option<ApiKey>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_key_hash(&self, key_hash: Vec<u8>) -> Result<Option<ApiKey>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::KeyHash.eq(key_hash))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_many(&self, filter: ApiKeyFilter) -> Result<Vec<ApiKey>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .order_by(Column::CreatedAt, filter.order_direction.into())
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert(&self, api_key: ApiKey) -> Result<ApiKey, DatabaseError> {
        let model = ActiveModel {
            user_id: Set(api_key.user_id),
            key_hash: Set(api_key.key_hash),
            permissions: Set(api_key.permissions.iter().map(|p| p.to_string()).collect()),
            description: Set(api_key.description),
            expires_at: Set(api_key.expires_at),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn delete_many(&self, filter: ApiKeyFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
