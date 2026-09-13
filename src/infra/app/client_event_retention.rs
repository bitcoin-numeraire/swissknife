use std::{sync::Arc, time::Duration};

use tokio::time::MissedTickBehavior;
use tracing::{error, warn};

use crate::application::composition::AppServices;

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
                if let Err(error) = self.services.client_event.prune().await {
                    error!(%error, "Failed to prune expired client events");
                }
            }
        });
    }
}
