use std::cmp::min;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::StreamExt;
use http::Uri;
use native_tls::{Certificate, TlsConnector};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::{fs, time::sleep};
use tokio_tungstenite::tungstenite::ClientRequestBuilder;
use tokio_tungstenite::{connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, warn};

use crate::application::entities::AppServices;
use crate::application::errors::LightningError;
use crate::domains::bitcoin::{BitcoinWallet, BtcNetwork, BtcTransaction, OnchainSyncCursor};
use crate::infra::lightning::EventsListener;

use super::lnd_rest_client::read_macaroon;
use super::lnd_types::{InvoiceResponse, TransactionResponse};
use super::LndRestClientConfig;

pub struct LndWebsocketListener {
    config: LndRestClientConfig,
    macaroon: String,
    services: Arc<AppServices>,
    network: BtcNetwork,
}

impl LndWebsocketListener {
    pub async fn new(
        config: LndRestClientConfig,
        services: Arc<AppServices>,
        wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        let macaroon = read_macaroon(&config.macaroon_path)
            .await
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let network = wallet.network();

        Ok(Self {
            config,
            macaroon,
            services,
            network,
        })
    }

    async fn listen_invoices(&self) -> Result<(), LightningError> {
        let max_reconnect_delay = self.config.ws_max_reconnect_delay;
        let mut reconnect_delay = self.config.ws_min_reconnect_delay;

        loop {
            let result = self.connect_and_handle_invoices().await;

            if let Err(err) = result {
                match err {
                    LightningError::ParseConfig(msg)
                    | LightningError::ConnectWebsocket(msg)
                    | LightningError::TLSConfig(msg)
                    | LightningError::ReadCertificates(msg) => {
                        return Err(LightningError::Listener(msg));
                    }
                    _ => {
                        error!(%err, "WebSocket connection error");
                        sleep(reconnect_delay).await;
                        reconnect_delay = min(reconnect_delay * 2, max_reconnect_delay);
                    }
                }
            }
        }
    }

    async fn listen_transactions(&self) -> Result<(), LightningError> {
        self.services
            .bitcoin
            .sync()
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        let max_reconnect_delay = self.config.ws_max_reconnect_delay;
        let mut reconnect_delay = self.config.ws_min_reconnect_delay;

        loop {
            let result = self.connect_and_handle_transactions().await;

            if let Err(err) = result {
                match err {
                    LightningError::ParseConfig(msg)
                    | LightningError::ConnectWebsocket(msg)
                    | LightningError::TLSConfig(msg)
                    | LightningError::ReadCertificates(msg) => {
                        return Err(LightningError::Listener(msg));
                    }
                    _ => {
                        error!(%err, "WebSocket connection error");
                        sleep(reconnect_delay).await;
                        reconnect_delay = min(reconnect_delay * 2, max_reconnect_delay);
                    }
                }
            }
        }
    }

    async fn connect_and_handle_invoices(&self) -> Result<(), LightningError> {
        let invoices_endpoint = format!("wss://{}/v1/invoices/subscribe", self.config.host);
        let uri = Uri::from_str(&invoices_endpoint).map_err(|e| LightningError::ParseConfig(e.to_string()))?;
        let builder = ClientRequestBuilder::new(uri).with_header("Grpc-Metadata-Macaroon", &self.macaroon);

        let tls_connector = self.create_tls_connector().await?;

        let (ws_stream, _) = connect_async_tls_with_config(builder, None, false, tls_connector)
            .await
            .map_err(|e| LightningError::ConnectWebsocket(e.to_string()))?;

        debug!("Connected to LND WebSocket server");

        self.handle_invoice_messages(ws_stream).await;

        debug!("Disconnected from LND WebSocket server");

        Ok(())
    }

    async fn connect_and_handle_transactions(&self) -> Result<(), LightningError> {
        let endpoint = format!("wss://{}/v1/transactions/subscribe", self.config.host);
        let uri = Uri::from_str(&endpoint).map_err(|e| LightningError::ParseConfig(e.to_string()))?;
        let builder = ClientRequestBuilder::new(uri).with_header("Grpc-Metadata-Macaroon", &self.macaroon);

        let tls_connector = self.create_tls_connector().await?;

        let (ws_stream, _) = connect_async_tls_with_config(builder, None, false, tls_connector)
            .await
            .map_err(|e| LightningError::ConnectWebsocket(e.to_string()))?;

        debug!("Connected to LND WebSocket transaction server");

        self.handle_transaction_messages(ws_stream).await;

        debug!("Disconnected from LND WebSocket transaction server");

        Ok(())
    }

    async fn create_tls_connector(&self) -> Result<Option<Connector>, LightningError> {
        if let Some(ca_cert_path) = &self.config.ca_cert_path {
            let ca_certificate = read_ca(ca_cert_path)
                .await
                .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;
            let tls_connector = TlsConnector::builder()
                .add_root_certificate(ca_certificate)
                .danger_accept_invalid_certs(self.config.accept_invalid_certs)
                .danger_accept_invalid_hostnames(self.config.accept_invalid_hostnames)
                .build()
                .map_err(|e| LightningError::TLSConfig(e.to_string()))?;
            Ok(Some(Connector::NativeTls(tls_connector)))
        } else if self.config.accept_invalid_certs || self.config.accept_invalid_hostnames {
            let tls_connector = TlsConnector::builder()
                .danger_accept_invalid_certs(self.config.accept_invalid_certs)
                .danger_accept_invalid_hostnames(self.config.accept_invalid_hostnames)
                .build()
                .map_err(|e| LightningError::TLSConfig(e.to_string()))?;
            Ok(Some(Connector::NativeTls(tls_connector)))
        } else {
            Ok(None)
        }
    }

    async fn handle_invoice_messages(&self, mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) {
        while let Some(message) = ws_stream.next().await {
            match message {
                Ok(msg) => {
                    if msg.is_text() {
                        let text = msg.into_text().unwrap();
                        if let Err(e) = self.process_invoice_message(&text).await {
                            error!(%e, "Failed to process message");
                        }
                    } else if msg.is_close() {
                        debug!("WebSocket closed");
                        break;
                    }
                }
                Err(err) => {
                    error!(%err, "Error receiving message");
                    break;
                }
            }
        }
    }

    async fn handle_transaction_messages(&self, mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) {
        while let Some(message) = ws_stream.next().await {
            match message {
                Ok(msg) => {
                    if msg.is_text() {
                        let text = msg.into_text().unwrap();
                        if let Err(e) = self.process_transaction_message(&text).await {
                            error!(%e, "Failed to process transaction message");
                        }
                    } else if msg.is_close() {
                        debug!("WebSocket closed");
                        break;
                    }
                }
                Err(err) => {
                    error!(%err, "Error receiving message");
                    break;
                }
            }
        }
    }

    async fn process_invoice_message(&self, text: &str) -> anyhow::Result<()> {
        let value: Value = serde_json::from_str(text)?;

        if let Some(event) = value.get("result") {
            match serde_json::from_value::<InvoiceResponse>(event.clone()) {
                Ok(invoice) => {
                    if invoice.state.as_str() == "SETTLED" {
                        if let Err(err) = self.services.event.invoice_paid(invoice.into()).await {
                            warn!(%err, "Failed to process incoming payment");
                        }
                    }
                }
                Err(err) => {
                    error!(%err, "Failed to parse SubscribeInvoices event");
                }
            }
        }

        Ok(())
    }

    async fn process_transaction_message(&self, text: &str) -> anyhow::Result<()> {
        let value: Value = serde_json::from_str(text)?;

        if let Some(event) = value.get("result") {
            match serde_json::from_value::<TransactionResponse>(event.clone()) {
                Ok(transaction) => {
                    let transaction: BtcTransaction = transaction.into();
                    self.handle_transaction(transaction).await;
                }
                Err(err) => {
                    error!(%err, "Failed to parse SubscribeTransactions event");
                }
            }
        }

        Ok(())
    }

    async fn handle_transaction(&self, transaction: BtcTransaction) {
        // Filter outputs based on transaction direction:
        // - Incoming tx (deposit): only process outputs that are ours
        // - Outgoing tx (withdrawal): only process outputs that are NOT ours (skip change)
        let relevant_outputs = transaction.outputs.iter().filter(|output| {
            if transaction.is_outgoing {
                !output.is_ours // Withdrawal destinations (not our change)
            } else {
                output.is_ours // Deposits to our wallet
            }
        });

        for output in relevant_outputs {
            let result = if transaction.is_outgoing {
                self.services
                    .event
                    .onchain_withdrawal(transaction.withdrawal_event())
                    .await
            } else {
                self.services
                    .event
                    .onchain_deposit(transaction.deposit_event(output), self.network.into())
                    .await
            };

            if let Err(err) = result {
                error!(%err, "Failed to process onchain transaction");
            }
        }

        if let Some(block_height) = transaction.block_height.filter(|&h| h > 0) {
            if let Err(err) = self
                .services
                .system
                .set_onchain_cursor(OnchainSyncCursor::BlockHeight(block_height))
                .await
            {
                warn!(%err, "Failed to persist onchain cursor");
            }
        }
    }
}

#[async_trait]
impl EventsListener for LndWebsocketListener {
    async fn listen(&self) -> Result<(), LightningError> {
        tokio::try_join!(self.listen_invoices(), self.listen_transactions())?;
        Ok(())
    }
}

async fn read_ca(path: &str) -> anyhow::Result<Certificate> {
    let ca_file = fs::read(PathBuf::from(path)).await?;
    let ca_certificate = Certificate::from_pem(&ca_file)?;
    Ok(ca_certificate)
}
