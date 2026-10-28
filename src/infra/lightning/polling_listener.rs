use std::{sync::Arc, time::Duration};

use async_trait::async_trait;

use crate::{
    application::{composition::AppServices, errors::LightningError},
    infra::lightning::EventsListener,
};

/// Reconcile known SwissKnife payments and invoices against durable provider
/// history. The regular listener supervisor owns retries after any error.
pub struct PollingListener {
    pub services: Arc<AppServices>,
    pub interval: Duration,
}

#[async_trait]
impl EventsListener for PollingListener {
    async fn listen(&self) -> Result<(), LightningError> {
        loop {
            tokio::try_join!(self.services.invoice.sync(), self.services.payment.sync())
                .map_err(|e| LightningError::Listener(e.to_string()))?;
            tokio::time::sleep(self.interval).await;
        }
    }
}
