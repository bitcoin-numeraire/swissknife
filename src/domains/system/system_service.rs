use async_trait::async_trait;
use serde_json;
use std::sync::Arc;
use tracing::{debug, error, info, trace};

use crate::{
    application::{entities::AppStore, errors::ApplicationError},
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
        let value = self.store.config.find(ONCHAIN_CURSOR_KEY).await?;
        let Some(value) = value else {
            return Ok(None);
        };
        let cursor = serde_json::from_value(value)
            .map_err(|e| crate::application::errors::DataError::Malformed(e.to_string()))?;
        Ok(Some(cursor))
    }

    async fn set_onchain_cursor(&self, cursor: OnchainSyncCursor) -> Result<(), ApplicationError> {
        let value = serde_json::to_value(cursor)
            .map_err(|e| crate::application::errors::DataError::Malformed(e.to_string()))?;
        self.store.config.upsert(ONCHAIN_CURSOR_KEY, value).await?;
        Ok(())
    }
}
