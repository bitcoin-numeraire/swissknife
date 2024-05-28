use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
};
use uuid::Uuid;

use crate::domains::lightning::adapters::models::lightning_address::{ActiveModel, Column, Entity};
use crate::domains::lightning::adapters::repository::LightningAddressRepository;
use crate::domains::lightning::entities::LightningAddressFilter;
use crate::{application::errors::DatabaseError, domains::lightning::entities::LightningAddress};

use super::LightningStore;

#[async_trait]
impl LightningAddressRepository for LightningStore {
    async fn find_address(&self, id: Uuid) -> Result<Option<LightningAddress>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

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
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

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
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_addresses(
        &self,
        filter: LightningAddressFilter,
    ) -> Result<Vec<LightningAddress>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.username, |q, username| {
                q.filter(Column::Username.eq(username))
            })
            .apply_if(filter.id, |q, id| q.filter(Column::Id.eq(id)))
            .order_by_desc(Column::CreatedAt)
            .offset(filter.pagination.offset)
            .limit(filter.pagination.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

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

    async fn delete_addresses(&self, filter: LightningAddressFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.id, |q, id| q.filter(Column::Id.eq(id)))
            .apply_if(filter.username, |q, username| {
                q.filter(Column::Username.eq(username))
            })
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
