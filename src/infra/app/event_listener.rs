use std::{sync::Arc, time::Duration};

use tokio::time::{sleep, Instant};
use tracing::{error, warn};

use crate::{
    application::{
        composition::AppServices,
        composition::{AppConfig, LightningProvider},
        errors::{ApplicationError, ConfigError},
    },
    domains::bitcoin::BitcoinWallet,
    infra::lightning::{
        cln::{ClnGrpcListener, ClnWebsocketListener},
        lnd::{LndGrpcListener, LndWebsocketListener},
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
}

impl EventListener {
    pub async fn new(
        config: AppConfig,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
        services: Arc<AppServices>,
    ) -> Result<Self, ApplicationError> {
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

                let listener = ClnWebsocketListener::new(cln_config, services.clone(), bitcoin_wallet).await?;

                Arc::new(listener) as Arc<dyn EventsListener>
            }
            LightningProvider::LndRest => {
                let lnd_rest_config = config
                    .lnd_rest_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                let listener = LndWebsocketListener::new(lnd_rest_config, services.clone(), bitcoin_wallet).await?;

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

        Ok(Self { listener, services })
    }

    pub async fn start(&self) -> Result<(), ApplicationError> {
        let listener = self.listener.clone();
        tokio::spawn(async move {
            let mut backoff = LISTENER_MIN_RECONNECT_DELAY;

            loop {
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
                backoff = if uptime >= LISTENER_STABLE_THRESHOLD {
                    LISTENER_MIN_RECONNECT_DELAY
                } else {
                    (backoff * 2).min(LISTENER_MAX_RECONNECT_DELAY)
                };
            }
        });

        tokio::try_join!(self.services.invoice.sync(), self.services.payment.sync())?;

        Ok(())
    }
}
