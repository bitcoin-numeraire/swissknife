use async_trait::async_trait;
use serde_json::{self, from_value, to_value};
use std::sync::Arc;
use tracing::{debug, error, info, trace};

use crate::{
    application::{
        composition::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::{account::PASSWORD_HASH_KEY, bitcoin::OnchainSyncCursor},
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

    use crate::{
        application::{
            composition::MockAppStoreBuilder,
            errors::{DatabaseError, LightningError},
        },
        infra::lightning::MockLnClient,
    };

    use super::*;

    fn service(store: MockAppStoreBuilder, ln_client: MockLnClient) -> SystemService {
        SystemService::new(store.build(), Arc::new(ln_client))
    }

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

                let health = service(store, ln_client).health_check().await;

                assert_eq!(health.database, HealthStatus::Operational);
                assert_eq!(health.ln_provider, HealthStatus::Operational);
                assert!(health.is_healthy);
            }
        }

        mod when_database_is_down {
            use super::*;

            #[tokio::test]
            async fn reports_database_unavailable_and_unhealthy() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .health
                    .expect_ping()
                    .times(1)
                    .returning(|| Err(DatabaseError::Ping("down".to_string())));

                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_health()
                    .times(1)
                    .returning(|| Ok(HealthStatus::Operational));

                let health = service(store, ln_client).health_check().await;

                assert_eq!(health.database, HealthStatus::Unavailable);
                assert_eq!(health.ln_provider, HealthStatus::Operational);
                assert!(!health.is_healthy);
            }
        }

        mod when_lightning_provider_is_down {
            use super::*;

            #[tokio::test]
            async fn reports_provider_unavailable_and_unhealthy() {
                let mut store = MockAppStoreBuilder::new();
                store.health.expect_ping().times(1).returning(|| Ok(()));

                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_health()
                    .times(1)
                    .returning(|| Err(LightningError::HealthCheck("down".to_string())));

                let health = service(store, ln_client).health_check().await;

                assert_eq!(health.database, HealthStatus::Operational);
                assert_eq!(health.ln_provider, HealthStatus::Unavailable);
                assert!(!health.is_healthy);
            }
        }
    }

    mod setup_check {
        use super::*;

        #[tokio::test]
        async fn reports_completion_flags_from_config() {
            let mut store = MockAppStoreBuilder::new();
            // welcome_complete present, password_hash absent.
            store.config.expect_find().times(2).returning(|key| {
                if key == WELCOME_COMPLETE_KEY {
                    Ok(Some(true.into()))
                } else {
                    Ok(None)
                }
            });

            let service = service(store, MockLnClient::new());

            let info = service.setup_check().await.unwrap();

            assert!(info.welcome_complete);
            assert!(!info.sign_up_complete);
        }
    }

    mod mark_welcome_complete {
        use super::*;

        mod when_not_yet_completed {
            use super::*;

            #[tokio::test]
            async fn inserts_the_flag() {
                let mut store = MockAppStoreBuilder::new();
                store.config.expect_find().times(1).returning(|_| Ok(None));
                store
                    .config
                    .expect_insert()
                    .withf(|key, value| key == WELCOME_COMPLETE_KEY && value == &serde_json::Value::Bool(true))
                    .times(1)
                    .returning(|_, _| Ok(()));

                let service = service(store, MockLnClient::new());

                assert!(service.mark_welcome_complete().await.is_ok());
            }
        }

        mod when_already_completed {
            use super::*;

            #[tokio::test]
            async fn is_idempotent_and_does_not_insert() {
                let mut store = MockAppStoreBuilder::new();
                store.config.expect_find().times(1).returning(|_| Ok(Some(true.into())));
                // insert is intentionally not expected.

                let service = service(store, MockLnClient::new());

                assert!(service.mark_welcome_complete().await.is_ok());
            }
        }
    }

    mod get_onchain_cursor {
        use super::*;

        mod when_present_and_valid {
            use super::*;

            #[tokio::test]
            async fn returns_the_cursor() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(|_| Ok(Some(serde_json::to_value(OnchainSyncCursor::BlockHeight(42)).unwrap())));

                let service = service(store, MockLnClient::new());

                let cursor = service.get_onchain_cursor().await.unwrap();

                assert_eq!(cursor, Some(OnchainSyncCursor::BlockHeight(42)));
            }
        }

        mod when_absent {
            use super::*;

            #[tokio::test]
            async fn returns_none() {
                let mut store = MockAppStoreBuilder::new();
                store.config.expect_find().times(1).returning(|_| Ok(None));

                let service = service(store, MockLnClient::new());

                assert_eq!(service.get_onchain_cursor().await.unwrap(), None);
            }
        }

        mod when_stored_value_is_malformed {
            use super::*;

            #[tokio::test]
            async fn returns_malformed_error() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .config
                    .expect_find()
                    .times(1)
                    .returning(|_| Ok(Some(serde_json::Value::String("not a cursor".to_string()))));

                let service = service(store, MockLnClient::new());

                let err = service.get_onchain_cursor().await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Malformed(_))));
            }
        }
    }

    mod set_onchain_cursor {
        use super::*;

        #[tokio::test]
        async fn upserts_the_serialized_cursor() {
            let mut store = MockAppStoreBuilder::new();
            store
                .config
                .expect_upsert()
                .withf(|key, value| {
                    key == ONCHAIN_CURSOR_KEY
                        && *value == serde_json::to_value(OnchainSyncCursor::BlockHeight(7)).unwrap()
                })
                .times(1)
                .returning(|_, _| Ok(()));

            let service = service(store, MockLnClient::new());

            assert!(service
                .set_onchain_cursor(OnchainSyncCursor::BlockHeight(7))
                .await
                .is_ok());
        }
    }

    mod version {
        use super::*;

        #[tokio::test]
        async fn returns_cargo_package_version() {
            let service = service(MockAppStoreBuilder::new(), MockLnClient::new());

            let version = service.version();

            assert_eq!(version.version, env!("CARGO_PKG_VERSION"));
        }
    }
}
