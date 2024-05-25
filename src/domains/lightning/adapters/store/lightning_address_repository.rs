use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::domains::lightning::adapters::models::lightning_address::{ActiveModel, Column, Entity};
use crate::domains::lightning::adapters::repository::LightningAddressRepository;
use crate::{application::errors::DatabaseError, domains::lightning::entities::LightningAddress};

use super::LightningStore;

#[async_trait]
impl LightningAddressRepository for LightningStore {
    async fn find_address(&self, id: Uuid) -> Result<Option<LightningAddress>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_address_by_user_id(
        &self,
        user_id: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_address_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_addresses(
        &self,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, DatabaseError> {
        let models = Entity::find()
            .order_by_desc(Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindAll(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert_address(
        &self,
        user_id: &str,
        username: &str,
    ) -> Result<LightningAddress, DatabaseError> {
        let model = ActiveModel {
            user_id: Set(user_id.to_owned()),
            username: Set(username.to_owned()),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }
}
