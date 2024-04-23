use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};

use crate::{
    application::errors::DatabaseError,
    domains::lightning::adapters::repository::TransactionManager,
};

#[derive(Clone)]
pub struct LightningStore {
    pub db: DatabaseConnection,
}

impl LightningStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TransactionManager for LightningStore {
    async fn begin(&self) -> Result<DatabaseTransaction, DatabaseError> {
        self.db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))
    }
}
