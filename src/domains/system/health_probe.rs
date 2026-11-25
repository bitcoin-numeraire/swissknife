use async_trait::async_trait;

use super::HealthStatus;

pub const DATABASE_PROBE: &str = "database";
pub const LIGHTNING_PROBE: &str = "ln_provider";

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait HealthProbe: Send + Sync {
    fn name(&self) -> &'static str;
    async fn health(&self) -> HealthStatus;
}
