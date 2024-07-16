use async_trait::async_trait;

use super::{HealthCheck, VersionInfo};

#[async_trait]
pub trait SystemUseCases: Send + Sync {
    async fn health_check(&self) -> HealthCheck;
    fn version(&self) -> VersionInfo;
}
