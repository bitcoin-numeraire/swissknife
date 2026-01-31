use std::sync::Arc;

use tracing::{info, warn};

use crate::{
    application::{
        dtos::{AppConfig, LightningProvider},
        entities::AppServices,
        errors::{ApplicationError, ConfigError},
    },
    domains::bitcoin::BitcoinWallet,
    infra::lightning::{
        cln::{ClnGrpcListener, ClnRestListener},
        lnd::LndWebsocketListener,
        EventsListener,
    },
};

pub struct EventListener {
    listener: Option<Arc<dyn EventsListener>>,
    services: Arc<AppServices>,
    bitcoin_wallet: Arc<dyn BitcoinWallet>,
}

impl EventListener {
    pub async fn new(
        config: AppConfig,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
        services: Arc<AppServices>,
    ) -> Result<Self, ApplicationError> {
        let listener = match config.ln_provider {
            LightningProvider::Breez => None,
            LightningProvider::ClnGrpc => {
                let cln_config = config
                    .cln_grpc_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                Some(Arc::new(ClnGrpcListener::new(cln_config)) as Arc<dyn EventsListener>)
            }
            LightningProvider::ClnRest => {
                let cln_config = config
                    .cln_rest_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                Some(Arc::new(ClnRestListener::new(cln_config)) as Arc<dyn EventsListener>)
            }
            LightningProvider::Lnd => {
                let lnd_config = config
                    .lnd_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                let listener = LndWebsocketListener::new(lnd_config).await?;

                Some(Arc::new(listener) as Arc<dyn EventsListener>)
            }
        };

        Ok(Self {
            listener,
            services,
            bitcoin_wallet,
        })
    }

    pub async fn start(&self) -> Result<(), ApplicationError> {
        if let Some(listener) = self.listener.clone() {
            let bitcoin_wallet = self.bitcoin_wallet.clone();
            let events = self.services.event.clone();

            tokio::spawn(async move {
                if let Err(err) = listener.listen(events, bitcoin_wallet).await {
                    warn!(%err, "Lightning listener failed");
                }
            });
        }

        let (invoices_synced, payments_synced) =
            tokio::try_join!(self.services.invoice.sync(), self.services.payment.sync())?;

        info!(invoices_synced, payments_synced, "Event listeners synced successfully");
        Ok(())
    }
}
