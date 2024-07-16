use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::ln_address::entities::{LnAddress, LnAddressFilter},
};

#[async_trait]
pub trait LnAddressRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<LnAddress>, DatabaseError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<LnAddress>, DatabaseError>;
    async fn find_by_user_id(&self, user: Uuid) -> Result<Option<LnAddress>, DatabaseError>;
    async fn find_many(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, DatabaseError>;
    async fn insert(&self, user: Uuid, username: &str) -> Result<LnAddress, DatabaseError>;
    async fn delete_many(&self, filter: LnAddressFilter) -> Result<u64, DatabaseError>;
}
