use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::{entities::BitcoinWallet, errors::LightningError},
    domains::{bitcoin::BitcoinEventsUseCases, ln_node::LnEventsUseCases},
    infra::lightning::LnNodeListener,
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
impl LnNodeListener for LndWebsocketListener {
    async fn listen(
        &self,
        ln_events: Arc<dyn LnEventsUseCases>,
        bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError> {
        let network = bitcoin_wallet.network();
        tokio::try_join!(
            listen_invoices(self.config.clone(), self.macaroon.clone(), ln_events),
            listen_transactions(self.config.clone(), self.macaroon.clone(), bitcoin_events, network),
        )?;

        Ok(())
    }
}
