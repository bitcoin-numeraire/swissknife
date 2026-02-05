use std::sync::Arc;

use crate::{
    application::{
        dtos::{AppConfig, LightningProvider},
        entities::AppServices,
        errors::{ApplicationError, ConfigError},
    },
    domains::bitcoin::BitcoinWallet,
    infra::lightning::{
        cln::{ClnGrpcListener, ClnWebsocketListener},
        lnd::LndWebsocketListener,
        EventsListener,
    },
};

pub struct EventListener {
    listener: Option<Arc<dyn EventsListener>>,
    services: Arc<AppServices>,
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

                let listener = ClnGrpcListener::new(cln_config, services.clone(), bitcoin_wallet).await?;

                Some(Arc::new(listener) as Arc<dyn EventsListener>)
            }
            LightningProvider::ClnRest => {
                let cln_config = config
                    .cln_rest_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                let listener = ClnWebsocketListener::new(cln_config, services.event.clone(), bitcoin_wallet).await?;

                Some(Arc::new(listener) as Arc<dyn EventsListener>)
            }
            LightningProvider::Lnd => {
                let lnd_config = config
                    .lnd_config
                    .clone()
                    .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

                let listener = LndWebsocketListener::new(lnd_config, services.event.clone(), bitcoin_wallet).await?;

                Some(Arc::new(listener) as Arc<dyn EventsListener>)
            }
        };

        Ok(Self { listener, services })
    }

    pub async fn start(&self) -> Result<(), ApplicationError> {
        if let Some(listener) = self.listener.clone() {
            let listener_task = listener.clone();
            tokio::spawn(async move {
                if let Err(err) = listener_task.listen().await {
                    panic!("Critical: Lightning listener failed: {}", err);
                }
            });
        }

        tokio::try_join!(
            self.services.invoice.sync(),
            self.services.payment.sync(),
            self.services.bitcoin.sync()
        )?;

        Ok(())
    }
}
