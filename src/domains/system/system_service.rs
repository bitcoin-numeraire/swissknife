use async_trait::async_trait;
use futures_util::future::join_all;
use serde_json::{self, from_value, to_value};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info, trace};

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{account::LOCAL_AUTH_INITIALIZED_KEY, bitcoin::OnchainSyncCursor},
};

use super::{
    ConfigRepository, HealthCheck, HealthProbe, HealthStatus, SetupInfo, SystemUseCases, VersionInfo, DATABASE_PROBE,
    LIGHTNING_PROBE,
};

const WELCOME_COMPLETE_KEY: &str = "welcome_complete";
const ONCHAIN_CURSOR_KEY: &str = "onchain_sync_cursor";

pub struct SystemService {
    config: Arc<dyn ConfigRepository>,
    probes: Vec<Arc<dyn HealthProbe>>,
}

impl SystemService {
    pub fn new(config: Arc<dyn ConfigRepository>, probes: Vec<Arc<dyn HealthProbe>>) -> Self {
        Self { config, probes }
    }
}

#[async_trait]
impl SystemUseCases for SystemService {
    async fn health_check(&self) -> HealthCheck {
        trace!("Checking system health");

        let mut statuses: HashMap<_, _> = join_all(
            self.probes
                .iter()
                .map(|probe| async { (probe.name(), probe.health().await) }),
        )
        .await
        .into_iter()
        .collect();

        let mut check = HealthCheck::new(
            statuses.remove(DATABASE_PROBE).unwrap_or(HealthStatus::Unavailable),
            statuses.remove(LIGHTNING_PROBE).unwrap_or(HealthStatus::Unavailable),
        );
        check.is_healthy &= statuses.values().all(|status| *status == HealthStatus::Operational);
        check
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

        let welcome_complete = self.config.find(WELCOME_COMPLETE_KEY).await?.is_some();
        let sign_up_complete: bool = self.config.find(LOCAL_AUTH_INITIALIZED_KEY).await?.is_some();

        debug!(%welcome_complete, %sign_up_complete, "Checking system setup");
        Ok(SetupInfo {
            welcome_complete,
            sign_up_complete,
        })
    }

    async fn mark_welcome_complete(&self) -> Result<(), ApplicationError> {
        debug!("Marking welcome flow as completed");

        if self.config.find(WELCOME_COMPLETE_KEY).await?.is_some() {
            return Ok(());
        }

        self.config.insert(WELCOME_COMPLETE_KEY, true.into()).await?;

        info!("Welcome flow marked as complete successfully");
        Ok(())
    }

    async fn get_onchain_cursor(&self) -> Result<Option<OnchainSyncCursor>, ApplicationError> {
        trace!("Retrieving onchain sync cursor");

        let Some(value) = self.config.find(ONCHAIN_CURSOR_KEY).await? else {
            return Ok(None);
        };

        let cursor = from_value(value).map_err(|e| DataError::Malformed(e.to_string()))?;

        debug!(?cursor, "Onchain sync cursor retrieved successfully");
        Ok(Some(cursor))
    }

    async fn set_onchain_cursor(&self, cursor: OnchainSyncCursor) -> Result<(), ApplicationError> {
        trace!("Setting onchain sync cursor");

        let value = to_value(cursor.clone()).map_err(|e| DataError::Malformed(e.to_string()))?;
        self.config.upsert(ONCHAIN_CURSOR_KEY, value).await?;

        debug!(?cursor, "Onchain sync cursor updated successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::application::composition::MockAppStoreBuilder;
    use crate::domains::system::MockHealthProbe;

    fn service(store: MockAppStoreBuilder) -> SystemService {
        SystemService::new(Arc::new(store.config), vec![])
    }

    mod health_check {
        use super::*;
        use tokio::sync::Barrier;

        struct Probe(&'static str, HealthStatus, Arc<Barrier>);
        #[async_trait]
        impl HealthProbe for Probe {
            fn name(&self) -> &'static str {
                self.0
            }
            async fn health(&self) -> HealthStatus {
                self.2.wait().await;
                self.1.clone()
            }
        }

        #[tokio::test]
        async fn probes_run_concurrently_and_keep_the_public_health_fields() {
            for (database, lightning, additional, healthy) in [
                (
                    HealthStatus::Operational,
                    HealthStatus::Operational,
                    HealthStatus::Operational,
                    true,
                ),
                (
                    HealthStatus::Unavailable,
                    HealthStatus::Operational,
                    HealthStatus::Operational,
                    false,
                ),
                (
                    HealthStatus::Operational,
                    HealthStatus::Maintenance,
                    HealthStatus::Operational,
                    false,
                ),
                (
                    HealthStatus::Operational,
                    HealthStatus::Operational,
                    HealthStatus::Unavailable,
                    false,
                ),
            ] {
                let barrier = Arc::new(Barrier::new(3));
                let probes: Vec<Arc<dyn HealthProbe>> = vec![
                    Arc::new(Probe("other", additional, barrier.clone())),
                    Arc::new(Probe(LIGHTNING_PROBE, lightning.clone(), barrier.clone())),
                    Arc::new(Probe(DATABASE_PROBE, database.clone(), barrier)),
                ];
                let service = SystemService::new(Arc::new(MockAppStoreBuilder::new().config), probes);
                let health = tokio::time::timeout(std::time::Duration::from_secs(2), service.health_check())
                    .await
                    .unwrap();
                assert_eq!(health.database, database);
                assert_eq!(health.ln_provider, lightning);
                assert_eq!(health.is_healthy, healthy);
            }
        }

        #[tokio::test]
        async fn a_missing_required_probe_is_unavailable() {
            let mut probe = MockHealthProbe::new();
            probe.expect_name().return_const(DATABASE_PROBE);
            probe.expect_health().returning(|| HealthStatus::Operational);
            let service = SystemService::new(Arc::new(MockAppStoreBuilder::new().config), vec![Arc::new(probe)]);
            let health = service.health_check().await;
            assert_eq!(health.ln_provider, HealthStatus::Unavailable);
            assert!(!health.is_healthy);
        }
    }

    mod setup_check {
        use super::*;

        #[tokio::test]
        async fn reports_completion_flags_from_config() {
            let mut store = MockAppStoreBuilder::new();
            // welcome_complete present, local_auth_initialized absent.
            store.config.expect_find().times(2).returning(|key| {
                if key == WELCOME_COMPLETE_KEY {
                    Ok(Some(true.into()))
                } else {
                    Ok(None)
                }
            });

            let service = service(store);

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

                let service = service(store);

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

                let service = service(store);

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

                let service = service(store);

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

                let service = service(store);

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

                let service = service(store);

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

            let service = service(store);

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
            let service = service(MockAppStoreBuilder::new());

            let version = service.version();

            assert_eq!(version.version, env!("CARGO_PKG_VERSION"));
        }
    }
}
