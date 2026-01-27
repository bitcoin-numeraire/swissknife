use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rust_socketio::asynchronous::Client as WsClient;

use crate::{
    application::{entities::EventsUseCases, errors::LightningError},
    domains::bitcoin::BitcoinWallet,
    infra::lightning::EventsListener,
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
impl EventsListener for ClnGrpcListener {
    async fn listen( &self,events: Arc<dyn EventsUseCases>, _bitcoin_wallet: Arc<dyn BitcoinWallet>) -> Result<(), LightningError> {
        let client = ClnGrpcClient::connect(&self.config).await?;
        listen_invoices(client, events, self.config.retry_delay)
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
impl EventsListener for ClnRestListener {
    async fn listen( &self,  events: Arc<dyn EventsUseCases>,  bitcoin_wallet: Arc<dyn BitcoinWallet>) -> Result<(), LightningError> {
        let network = bitcoin_wallet.network();
        let ws_client = connect_websocket(self.config.clone(), events, network).await?;
        let mut guard = self
            .ws_client
            .lock()
            .map_err(|_| LightningError::Listener("Failed to lock websocket listener".to_string()))?;
        *guard = Some(ws_client);
        Ok(())
    }
}
