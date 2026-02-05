use async_trait::async_trait;

use crate::application::errors::ApplicationError;

use super::{HealthCheck, SetupInfo, VersionInfo};

#[async_trait]
pub trait SystemUseCases: Send + Sync {
    async fn health_check(&self) -> HealthCheck;
    fn version(&self) -> VersionInfo;
    async fn setup_check(&self) -> Result<SetupInfo, ApplicationError>;
    async fn mark_welcome_complete(&self) -> Result<(), ApplicationError>;
    async fn get_onchain_cursor(&self) -> Result<Option<crate::domains::bitcoin::OnchainSyncCursor>, ApplicationError>;
    async fn set_onchain_cursor(
        &self,
        cursor: crate::domains::bitcoin::OnchainSyncCursor,
    ) -> Result<(), ApplicationError>;
}
