use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{bitcoin::BitcoinWallet, event::EventUseCases},
    infra::lightning::EventsListener,
};

use super::{
    lnd_rest_client::read_macaroon,
    lnd_websocket_client::{listen_invoices, listen_transactions},
    LndRestClientConfig,
};

pub struct LndWebsocketListener {
    config: LndRestClientConfig,
    macaroon: String,
}

impl LndWebsocketListener {
    pub async fn new(config: LndRestClientConfig) -> Result<Self, LightningError> {
        let macaroon = read_macaroon(&config.macaroon_path)
            .await
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        Ok(Self { config, macaroon })
    }
}

#[async_trait]
impl EventsListener for LndWebsocketListener {
    async fn listen(
        &self,
        events: Arc<dyn EventUseCases>,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError> {
        let network = bitcoin_wallet.network();
        tokio::try_join!(
            listen_invoices(self.config.clone(), self.macaroon.clone(), events.clone()),
            listen_transactions(self.config.clone(), self.macaroon.clone(), events, network),
        )?;

        Ok(())
    }
}
