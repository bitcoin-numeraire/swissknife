use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{bitcoin::BitcoinWallet, event::EventUseCases},
    infra::lightning::EventsListener,
};

use super::{
    cln_grpc_client::{ClnClientConfig, ClnGrpcClient},
    cln_grpc_listener::listen_invoices,
};

pub struct ClnGrpcListener {
    config: ClnClientConfig,
}

impl ClnGrpcListener {
    pub fn new(config: ClnClientConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EventsListener for ClnGrpcListener {
    async fn listen(
        &self,
        events: Arc<dyn EventUseCases>,
        _bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError> {
        let client = ClnGrpcClient::connect(&self.config).await?;
        listen_invoices(client, events, self.config.retry_delay)
            .await
            .map_err(|err| LightningError::Listener(err.to_string()))
    }
}
