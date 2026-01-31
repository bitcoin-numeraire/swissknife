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

use crate::application::errors::LightningError;
use crate::domains::bitcoin::{BitcoinWallet, BtcNetwork, BtcTransaction};
use crate::domains::event::EventUseCases;
use crate::infra::lightning::EventsListener;

use super::lnd_rest_client::read_macaroon;
use super::lnd_types::{InvoiceResponse, TransactionResponse};
use super::LndRestClientConfig;

pub struct LndWebsocketListener {
    config: LndRestClientConfig,
    macaroon: String,
    events: Arc<dyn EventUseCases>,
    network: BtcNetwork,
}

impl LndWebsocketListener {
    pub async fn new(
        config: LndRestClientConfig,
        events: Arc<dyn EventUseCases>,
        wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        let macaroon = read_macaroon(&config.macaroon_path)
            .await
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let network = wallet.network();

        Ok(Self {
            config,
            macaroon,
            events,
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
                        if let Err(err) = self.events.invoice_paid(invoice.into()).await {
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

                    for output in transaction.outputs.iter() {
                        let output_event = transaction.output_event(output);
                        let result = if output.is_ours {
                            self.events.onchain_deposit(output_event, self.network.into()).await
                        } else {
                            self.events.onchain_withdrawal(output_event).await
                        };

                        if let Err(err) = result {
                            error!(%err, "Failed to process onchain transaction");
                        }
                    }
                }
                Err(err) => {
                    error!(%err, "Failed to parse SubscribeTransactions event");
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventsListener for LndWebsocketListener {
    async fn listen(
        &self,
        _events: Arc<dyn EventUseCases>,
        _bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError> {
        tokio::try_join!(self.listen_invoices(), self.listen_transactions())?;
        Ok(())
    }
}

async fn read_ca(path: &str) -> anyhow::Result<Certificate> {
    let ca_file = fs::read(PathBuf::from(path)).await?;
    let ca_certificate = Certificate::from_pem(&ca_file)?;
    Ok(ca_certificate)
}
