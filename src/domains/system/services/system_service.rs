use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, trace};

use crate::{
    application::entities::AppStore,
    domains::system::entities::{HealthCheck, HealthStatus, VersionInfo},
    infra::lightning::LnClient,
};

use super::SystemUseCases;

pub struct SystemService {
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
}

impl SystemService {
    pub fn new(store: AppStore, ln_client: Arc<dyn LnClient>) -> Self {
        SystemService { store, ln_client }
    }
}

#[async_trait]
impl SystemUseCases for SystemService {
    async fn health_check(&self) -> HealthCheck {
        trace!("Checking system health");

        let database = self
            .store
            .ping()
            .await
            .map(|_| HealthStatus::Operational)
            .unwrap_or_else(|e| {
                error!("Database health check failed: {:?}", e);
                HealthStatus::Unavailable
            });

        let ln_provider = self
            .ln_client
            .health()
            .await
            .map(|status| status)
            .unwrap_or_else(|e| {
                error!("Lightning provider health check failed: {:?}", e);
                HealthStatus::Unavailable
            });

        HealthCheck {
            database,
            ln_provider,
        }
    }

    fn version(&self) -> VersionInfo {
        trace!("Checking system version");

        VersionInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_time: env!("BUILD_TIME").to_string(),
        }
    }
}
