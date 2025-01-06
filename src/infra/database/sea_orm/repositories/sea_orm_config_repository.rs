use crate::{
    application::errors::DatabaseError,
    domains::system::ConfigRepository,
    infra::database::sea_orm::models::config::{ActiveModel, Entity},
};
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::Value;

#[derive(Clone)]
pub struct SeaOrmConfigRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmConfigRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ConfigRepository for SeaOrmConfigRepository {
    async fn find(&self, key: &str) -> Result<Option<Value>, DatabaseError> {
        let model = Entity::find_by_id(key)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        if let Some(m) = model {
            return Ok(m.value);
        }

        Ok(None)
    }

    async fn insert(&self, key: &str, value: Value) -> Result<(), DatabaseError> {
        let model = ActiveModel {
            key: Set(key.to_string()),
            value: Set(Some(value)),
        };

        model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(())
    }
}
