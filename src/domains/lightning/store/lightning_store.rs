use async_trait::async_trait;

use crate::{application::errors::DatabaseError, domains::lightning::entities::LightningAddress};

#[async_trait]
pub trait LightningAddressRepository: Sync + Send {
    async fn get(&self, username: &str) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn list(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn insert(&self, user: &str, username: &str) -> Result<LightningAddress, DatabaseError>;
}
