use async_trait::async_trait;

use crate::{application::errors::DatabaseError, domains::lightning::entities::LightningAddress};

#[async_trait]
pub trait LightningAddressRepository: Sync + Send {
    async fn get_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn get_by_user_id(&self, user: &str) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn list(
        &self,
        user: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn insert(&self, user: &str, username: &str) -> Result<LightningAddress, DatabaseError>;
}
