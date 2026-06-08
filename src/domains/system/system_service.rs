use async_trait::async_trait;
use serde_json::{self, from_value, to_value};
use std::sync::Arc;
use tracing::{debug, error, info, trace};

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::{bitcoin::OnchainSyncCursor, user::PASSWORD_HASH_KEY},
    infra::lightning::LnClient,
};

use super::{HealthCheck, HealthStatus, SetupInfo, SystemUseCases, VersionInfo};

const WELCOME_COMPLETE_KEY: &str = "welcome_complete";
const ONCHAIN_CURSOR_KEY: &str = "onchain_sync_cursor";

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
            .health
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

        let welcome_complete = self.store.config.find(WELCOME_COMPLETE_KEY).await?.is_some();
        let sign_up_complete: bool = self.store.config.find(PASSWORD_HASH_KEY).await?.is_some();

        debug!(%welcome_complete, %sign_up_complete, "Checking system setup");
        Ok(SetupInfo {
            welcome_complete,
            sign_up_complete,
        })
    }

    async fn mark_welcome_complete(&self) -> Result<(), ApplicationError> {
        debug!("Marking welcome flow as completed");

        if self.store.config.find(WELCOME_COMPLETE_KEY).await?.is_some() {
            return Ok(());
        }

        self.store.config.insert(WELCOME_COMPLETE_KEY, true.into()).await?;

        info!("Welcome flow marked as complete successfully");
        Ok(())
    }

    async fn get_onchain_cursor(&self) -> Result<Option<OnchainSyncCursor>, ApplicationError> {
        trace!("Retrieving onchain sync cursor");

        let Some(value) = self.store.config.find(ONCHAIN_CURSOR_KEY).await? else {
            return Ok(None);
        };

        let cursor = from_value(value).map_err(|e| DataError::Malformed(e.to_string()))?;

        debug!(?cursor, "Onchain sync cursor retrieved successfully");
        Ok(Some(cursor))
    }

    async fn set_onchain_cursor(&self, cursor: OnchainSyncCursor) -> Result<(), ApplicationError> {
        trace!("Setting onchain sync cursor");

        let value = to_value(cursor.clone()).map_err(|e| DataError::Malformed(e.to_string()))?;
        self.store.config.upsert(ONCHAIN_CURSOR_KEY, value).await?;

        debug!(?cursor, "Onchain sync cursor updated successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{application::entities::MockAppStoreBuilder, infra::lightning::MockLnClient};

    use super::*;

    mod health_check {
        use super::*;

        mod when_dependencies_are_operational {
            use super::*;

            #[tokio::test]
            async fn reports_system_healthy() {
                let mut store = MockAppStoreBuilder::new();
                store.health.expect_ping().times(1).returning(|| Ok(()));

                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_health()
                    .times(1)
                    .returning(|| Ok(HealthStatus::Operational));

                let service = SystemService::new(store.build(), Arc::new(ln_client));

                let health = service.health_check().await;

                assert_eq!(health.database, HealthStatus::Operational);
                assert_eq!(health.ln_provider, HealthStatus::Operational);
                assert!(health.is_healthy);
            }
        }
    }
}
