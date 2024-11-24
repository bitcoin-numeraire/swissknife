use async_trait::async_trait;

use crate::application::errors::ApplicationError;

use super::{HealthCheck, SetupInfo, VersionInfo};

#[async_trait]
pub trait SystemUseCases: Send + Sync {
    async fn health_check(&self) -> HealthCheck;
    fn version(&self) -> VersionInfo;
    async fn setup_check(&self) -> Result<SetupInfo, ApplicationError>;
}
