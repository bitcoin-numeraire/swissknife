use crate::application::errors::DatabaseError;
use async_trait::async_trait;
use swissknife_types::{AgentKey, WalletCapabilities};
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AgentKeyRepository: Send + Sync {
    async fn insert(&self, account: Uuid, key: AgentKey, digest: Vec<u8>) -> Result<(), DatabaseError>;
    async fn list(&self, account: Uuid, wallet: Uuid) -> Result<Vec<AgentKey>, DatabaseError>;
    async fn revoke(&self, account: Uuid, wallet: Uuid, key: Uuid) -> Result<bool, DatabaseError>;
    async fn active_grant(
        &self,
        wallet: Uuid,
        digest: Vec<u8>,
    ) -> Result<Option<(Uuid, WalletCapabilities)>, DatabaseError>;
}
