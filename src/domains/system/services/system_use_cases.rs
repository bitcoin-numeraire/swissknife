use async_trait::async_trait;

use crate::domains::system::entities::{HealthCheck, VersionInfo};

#[async_trait]
pub trait SystemUseCases: Send + Sync {
    async fn health_check(&self) -> HealthCheck;
    fn version(&self) -> VersionInfo;
}
