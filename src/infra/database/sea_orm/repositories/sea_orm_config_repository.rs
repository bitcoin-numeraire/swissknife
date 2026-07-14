use crate::{
    application::errors::DatabaseError,
    domains::system::ConfigRepository,
    infra::database::sea_orm::models::{config, config::ActiveModel, prelude::Config},
};
use async_trait::async_trait;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryTrait, Set,
};
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
        let model = Config::find_by_id(key)
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

    async fn insert_if_absent(&self, key: &str, value: Value) -> Result<bool, DatabaseError> {
        let model = ActiveModel {
            key: Set(key.to_string()),
            value: Set(Some(value)),
        };
        let statement = Config::insert(model)
            .on_conflict(OnConflict::column(config::Column::Key).do_nothing().to_owned())
            .build(self.db.get_database_backend());
        let result = self
            .db
            .execute(statement)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(result.rows_affected() == 1)
    }

    async fn upsert(&self, key: &str, value: Value) -> Result<(), DatabaseError> {
        if let Some(existing) = Config::find_by_id(key)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
        {
            let mut active_model: ActiveModel = existing.into();
            active_model.value = Set(Some(value));
            active_model
                .update(&self.db)
                .await
                .map_err(|e| DatabaseError::Update(e.to_string()))?;
            return Ok(());
        }

        self.insert(key, value).await
    }
}
