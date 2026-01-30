use std::sync::Arc;

use tracing::{info, warn};

use crate::{
    application::{
        dtos::{AppConfig, LightningProvider},
        entities::{AppAdapters, AppServices},
        errors::{ApplicationError, ConfigError},
    },
    infra::lightning::{
        cln::{ClnGrpcListener, ClnRestListener},
        lnd::LndWebsocketListener,
        EventsListener,
    },
};

pub struct EventListener {
    ln_listener: Option<Arc<dyn EventsListener>>,
    services: Arc<AppServices>,
    adapters: AppAdapters,
}

impl EventListener {
    pub async fn new(
        config: AppConfig,
        adapters: AppAdapters,
        services: Arc<AppServices>,
    ) -> Result<Self, ApplicationError> {
        let ln_listener = build_ln_listener(&config).await?;

        Ok(Self {
            ln_listener,
            services,
            adapters,
        })
    }

    pub async fn start(&self) -> Result<(), ApplicationError> {
        if let Some(listener) = self.ln_listener.clone() {
            let bitcoin_wallet = self.adapters.bitcoin_wallet.clone();
            let events = self.services.event.clone();

            tokio::spawn(async move {
                if let Err(err) = listener.listen(events, bitcoin_wallet).await {
                    warn!(%err, "Lightning listener failed");
                }
            });
        }

        if self.ln_listener.is_some() {
            let (invoices_synced, payments_synced) =
                tokio::try_join!(self.services.invoice.sync(), self.services.payment.sync(),)?;

            info!(invoices_synced, payments_synced, "Event listeners synced successfully");
        } else {
            info!("Event listener sync skipped for provider without external listener");
        }

        Ok(())
    }
}

async fn build_ln_listener(config: &AppConfig) -> Result<Option<Arc<dyn EventsListener>>, ApplicationError> {
    match config.ln_provider {
        LightningProvider::Breez => Ok(None),
        LightningProvider::ClnGrpc => {
            let cln_config = config
                .cln_grpc_config
                .clone()
                .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;
            Ok(Some(Arc::new(ClnGrpcListener::new(cln_config))))
        }
        LightningProvider::ClnRest => {
            let cln_config = config
                .cln_rest_config
                .clone()
                .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;
            Ok(Some(Arc::new(ClnRestListener::new(cln_config))))
        }
        LightningProvider::Lnd => {
            let lnd_config = config
                .lnd_config
                .clone()
                .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;
            let listener = LndWebsocketListener::new(lnd_config).await?;
            Ok(Some(Arc::new(listener)))
        }
    }
}
