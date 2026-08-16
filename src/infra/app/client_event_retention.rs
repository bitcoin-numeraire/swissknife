use std::{sync::Arc, time::Duration};

use tokio::time::MissedTickBehavior;
use tracing::{debug, error, warn};

use crate::application::{composition::AppServices, errors::ApplicationError};

pub struct ClientEventRetentionWorker {
    services: Arc<AppServices>,
    cleanup_interval: Duration,
}

impl ClientEventRetentionWorker {
    pub fn new(services: Arc<AppServices>, cleanup_interval: Duration) -> Self {
        Self {
            services,
            cleanup_interval,
        }
    }

    pub fn start(self) {
        if self.cleanup_interval.is_zero() {
            warn!("Client event cleanup is disabled because its interval is zero");
            return;
        }

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.cleanup_interval);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                interval.tick().await;
                match self.run_once().await {
                    Ok(0) => {}
                    Ok(pruned) => debug!(pruned, "Pruned expired client events"),
                    Err(error) => error!(%error, "Failed to prune expired client events"),
                }
            }
        });
    }

    async fn run_once(&self) -> Result<u64, ApplicationError> {
        self.services.client_event.prune().await
    }
}

#[cfg(test)]
mod tests {
    use crate::application::composition::MockAppServicesBuilder;

    use super::*;

    #[tokio::test]
    async fn run_once_prunes_through_the_client_event_service() {
        let mut services = MockAppServicesBuilder::new();
        services.client_event.expect_prune().times(1).return_once(|| Ok(3));
        let worker = ClientEventRetentionWorker::new(Arc::new(services.build()), Duration::from_secs(60));

        assert_eq!(worker.run_once().await.unwrap(), 3);
    }
}
