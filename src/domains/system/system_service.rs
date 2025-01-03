use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, trace};

use crate::{
    application::{entities::AppStore, errors::ApplicationError},
    infra::lightning::LnClient,
};

use super::{HealthCheck, HealthStatus, SetupInfo, SystemUseCases, VersionInfo};

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
            .unwrap_or_else(|err| {
                error!(%err, "Database health check failed");
                HealthStatus::Unavailable
            });

        let ln_provider = self.ln_client.health().await.unwrap_or_else(|err| {
            error!(%err, "Lightning provider health check failed");
            HealthStatus::Unavailable
        });

        HealthCheck::new(database, ln_provider)
    }

    fn version(&self) -> VersionInfo {
        trace!("Checking system version");

        VersionInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_time: env!("BUILD_TIME").to_string(),
        }
    }

    async fn setup_check(&self) -> Result<SetupInfo, ApplicationError> {
        trace!("Checking system setup");
        let n_wallets: u64 = self.store.wallet.count().await?;

        Ok(SetupInfo {
            complete: n_wallets > 0,
        })
    }
}
