use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rust_socketio::asynchronous::Client as WsClient;

use crate::{
    application::{entities::BitcoinWallet, errors::LightningError},
    domains::{bitcoin::BitcoinEventsUseCases, ln_node::LnEventsUseCases},
    infra::lightning::LnNodeListener,
};

use super::{
    cln_grpc_client::{ClnClientConfig, ClnGrpcClient},
    cln_grpc_listener::listen_invoices,
    cln_websocket_client::connect_websocket,
    ClnRestClientConfig,
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
impl LnNodeListener for ClnGrpcListener {
    async fn listen(
        &self,
        ln_events: Arc<dyn LnEventsUseCases>,
        _bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
        _bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError> {
        let client = ClnGrpcClient::connect(&self.config).await?;
        listen_invoices(client, ln_events, self.config.retry_delay)
            .await
            .map_err(|err| LightningError::Listener(err.to_string()))
    }
}

pub struct ClnRestListener {
    config: ClnRestClientConfig,
    ws_client: Mutex<Option<WsClient>>,
}

impl ClnRestListener {
    pub fn new(config: ClnRestClientConfig) -> Self {
        Self {
            config,
            ws_client: Mutex::new(None),
        }
    }
}

#[async_trait]
impl LnNodeListener for ClnRestListener {
    async fn listen(
        &self,
        ln_events: Arc<dyn LnEventsUseCases>,
        bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError> {
        let network = bitcoin_wallet.network();
        let ws_client = connect_websocket(self.config.clone(), ln_events, bitcoin_events, network).await?;
        let mut guard = self
            .ws_client
            .lock()
            .map_err(|_| LightningError::Listener("Failed to lock websocket listener".to_string()))?;
        *guard = Some(ws_client);
        Ok(())
    }
}
