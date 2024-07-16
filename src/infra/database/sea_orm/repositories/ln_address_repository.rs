use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait,
};
use uuid::Uuid;

use crate::application::errors::DatabaseError;
use crate::domains::ln_address::{LnAddress, LnAddressFilter, LnAddressRepository};
use crate::infra::database::sea_orm::models::ln_address::{ActiveModel, Column, Entity};

#[derive(Clone)]
pub struct SeaOrmLnAddressRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmLnAddressRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LnAddressRepository for SeaOrmLnAddressRepository {
    async fn find(&self, id: Uuid) -> Result<Option<LnAddress>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<LnAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<LnAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_many(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.username, |q, username| {
                q.filter(Column::Username.eq(username))
            })
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.active, |q, active| {
                q.filter(Column::Active.eq(active))
            })
            .order_by(Column::CreatedAt, filter.order_direction.into())
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert(&self, user_id: Uuid, username: &str) -> Result<LnAddress, DatabaseError> {
        let model = ActiveModel {
            user_id: Set(user_id),
            username: Set(username.to_owned()),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn delete_many(&self, filter: LnAddressFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.username, |q, username| {
                q.filter(Column::Username.eq(username))
            })
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
