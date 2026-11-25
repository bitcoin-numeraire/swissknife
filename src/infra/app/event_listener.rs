use std::{sync::Arc, time::Duration};

use tokio::{
    task::yield_now,
    time::{sleep, Instant},
};
use tracing::{error, warn};

use crate::{
    application::{
        composition::AppServices,
        composition::{AppConfig, LightningProvider},
        errors::{ApplicationError, ConfigError},
    },
    domains::bitcoin::BitcoinWallet,
    infra::lightning::{
        cln::{ClnGrpcListener, ClnRestListener},
        lnd::{LndGrpcListener, LndRestListener},
        EventsListener,
    },
};

/// Backoff bounds for the listener supervisor. The listener owns deposit/invoice
/// ingestion, so it must never stay down: on any exit we reconnect (and re-sync)
/// with exponential backoff rather than letting the task die silently (issue #267).
const LISTENER_MIN_RECONNECT_DELAY: Duration = Duration::from_secs(1);
const LISTENER_MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);
/// A connection that stayed up at least this long is considered healthy, so the
/// next failure restarts from the minimum delay instead of the grown backoff.
const LISTENER_STABLE_THRESHOLD: Duration = Duration::from_secs(60);

pub struct EventListener {
    listener: Arc<dyn EventsListener>,
    services: Arc<AppServices>,
    reconnect_delays: (Duration, Duration),
}

impl EventListener {
    pub async fn new(
        config: AppConfig,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
        services: Arc<AppServices>,
    ) -> Result<Self, ApplicationError> {
        let mut reconnect_delays = (LISTENER_MIN_RECONNECT_DELAY, LISTENER_MAX_RECONNECT_DELAY);
        let listener = match config.ln_provider {
            LightningProvider::ClnGrpc => {
                let cln_config = config
                    .cln_grpc_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                let listener = ClnGrpcListener::new(cln_config, services.clone(), bitcoin_wallet).await?;

                Arc::new(listener) as Arc<dyn EventsListener>
            }
            LightningProvider::ClnRest => {
                let cln_config = config
                    .cln_rest_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                let min = cln_config.ws_min_reconnect_delay.max(Duration::from_millis(100));
                reconnect_delays = (min, cln_config.ws_max_reconnect_delay.max(min));
                let listener = ClnRestListener::new(cln_config, services.clone()).await?;

                Arc::new(listener) as Arc<dyn EventsListener>
            }
            LightningProvider::LndRest => {
                let lnd_rest_config = config
                    .lnd_rest_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                let listener = LndRestListener::new(lnd_rest_config, services.clone()).await?;

                Arc::new(listener) as Arc<dyn EventsListener>
            }
            LightningProvider::LndGrpc => {
                let lnd_grpc_config = config
                    .lnd_grpc_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                let listener = LndGrpcListener::new(lnd_grpc_config, services.clone(), bitcoin_wallet).await?;

                Arc::new(listener) as Arc<dyn EventsListener>
            }
        };

        Ok(Self {
            listener,
            services,
            reconnect_delays,
        })
    }

    pub async fn start(&self) -> Result<(), ApplicationError> {
        let listener = self.listener.clone();
        let services = self.services.clone();
        let (min_delay, max_delay) = self.reconnect_delays;
        tokio::spawn(async move {
            let mut backoff = min_delay;
            let mut replay_before_listen = false;

            loop {
                if replay_before_listen {
                    if let Err(err) = Self::sync_offchain_state(&services).await {
                        error!(%err, ?backoff, "Lightning listener replay sync failed; retrying");
                        sleep(backoff).await;
                        backoff = backoff.saturating_mul(2).min(max_delay);
                        continue;
                    }
                }

                let started = Instant::now();
                let outcome = listener.listen().await;
                let uptime = started.elapsed();

                match outcome {
                    // `listen()` only returns on failure (or a clean stream close); either
                    // way the listener is no longer ingesting events, so reconnect.
                    Ok(()) => warn!("Lightning listener stopped; reconnecting"),
                    Err(err) => error!(%err, ?backoff, "Lightning listener failed; reconnecting"),
                }

                sleep(backoff).await;
                replay_before_listen = true;
                backoff = if uptime >= LISTENER_STABLE_THRESHOLD {
                    min_delay
                } else {
                    backoff.saturating_mul(2).min(max_delay)
                };
            }
        });

        yield_now().await;
        Self::sync_offchain_state(&self.services).await?;

        Ok(())
    }

    async fn sync_offchain_state(services: &AppServices) -> Result<(), ApplicationError> {
        let (synced_invoices, synced_payments) = tokio::try_join!(services.invoice.sync(), services.payment.sync())?;

        tracing::debug!(synced_invoices, synced_payments, "Lightning off-chain state replayed");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use crate::{application::composition::MockAppServicesBuilder, infra::lightning::MockEventsListener};

    use super::*;

    #[tokio::test]
    async fn sync_offchain_state_replays_invoices_and_payments() {
        let mut builder = MockAppServicesBuilder::new();
        builder.invoice.expect_sync().times(1).returning(|| Ok(2));
        builder.payment.expect_sync().times(1).returning(|| Ok(3));
        let services = builder.build();

        EventListener::sync_offchain_state(&services).await.unwrap();
    }

    #[tokio::test]
    async fn start_begins_listening_before_startup_replay() {
        let listening_started = Arc::new(AtomicBool::new(false));
        let mut listener = MockEventsListener::new();
        listener.expect_listen().times(1).returning({
            let listening_started = listening_started.clone();
            move || {
                listening_started.store(true, Ordering::SeqCst);
                Ok(())
            }
        });

        let mut builder = MockAppServicesBuilder::new();
        builder.invoice.expect_sync().times(1).returning({
            let listening_started = listening_started.clone();
            move || {
                assert!(listening_started.load(Ordering::SeqCst));
                Ok(2)
            }
        });
        builder.payment.expect_sync().times(1).returning(|| Ok(3));
        let services = Arc::new(builder.build());
        let event_listener = EventListener {
            listener: Arc::new(listener),
            services,
            reconnect_delays: (LISTENER_MIN_RECONNECT_DELAY, LISTENER_MAX_RECONNECT_DELAY),
        };

        event_listener.start().await.unwrap();
    }
}
