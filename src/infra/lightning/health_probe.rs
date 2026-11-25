use std::sync::Arc;

use async_trait::async_trait;
use tracing::error;

use super::LnClient;
use crate::domains::system::{HealthProbe, HealthStatus, LIGHTNING_PROBE};

pub struct LightningHealthProbe(pub Arc<dyn LnClient>);

#[async_trait]
impl HealthProbe for LightningHealthProbe {
    fn name(&self) -> &'static str {
        LIGHTNING_PROBE
    }

    async fn health(&self) -> HealthStatus {
        self.0.health().await.unwrap_or_else(|err| {
            error!(%err, "Lightning provider health check failed");
            HealthStatus::Unavailable
        })
    }
}
