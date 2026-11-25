use crate::application::composition::AppServices;
use std::{sync::Arc, time::Duration};

pub fn start_offer_worker(services: Arc<AppServices>) {
    tokio::spawn(async move {
        loop {
            match services.offer.sync().await {
                Ok(true) => continue,
                Ok(false) => {}
                Err(error) => tracing::warn!(%error,"Offer receipt synchronization will retry"),
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
}
