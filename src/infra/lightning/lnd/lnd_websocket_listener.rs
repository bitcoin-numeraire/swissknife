use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::StreamExt;
use http::Uri;
use native_tls::{Certificate, TlsConnector};
use serde_json::Value;
use tokio::fs;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::ClientRequestBuilder;
use tokio_tungstenite::{connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, trace, warn};

use crate::application::composition::AppServices;
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

    // Every error propagates out of `listen()`; the `EventListener` supervisor owns
    // reconnection (re-running `sync()` from the un-advanced cursor). A clean disconnect
    // is surfaced as an error so it takes the same reconnect path.
    async fn listen_invoices(&self) -> Result<(), LightningError> {
        self.connect_and_handle_invoices().await?;
        Err(LightningError::ConnectWebsocket(
            "LND invoice websocket disconnected".to_string(),
        ))
    }

    async fn listen_transactions(&self) -> Result<(), LightningError> {
        self.services
            .bitcoin
            .sync()
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        self.connect_and_handle_transactions().await?;
        Err(LightningError::ConnectWebsocket(
            "LND transaction websocket disconnected".to_string(),
        ))
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

        self.handle_invoice_messages(ws_stream).await?;

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

        self.handle_transaction_messages(ws_stream).await?;

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

    async fn handle_invoice_messages(
        &self,
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<(), LightningError> {
        while let Some(message) = ws_stream.next().await {
            match message {
                Ok(msg) => {
                    if msg.is_text() {
                        let text = msg.into_text().unwrap();
                        // [DEBUG-FLAKE] raw invoice frame as received from LND.
                        debug!(%text, "LND invoice frame received");
                        // Parse/semantic errors are skippable; database projection errors
                        // propagate so the supervisor replays pending state before resubscribe.
                        self.process_invoice_message(&text).await?;
                    } else if msg.is_close() {
                        debug!(?msg, "LND invoice websocket closed by peer");
                        return Ok(());
                    } else {
                        trace!(?msg, "LND invoice non-text frame");
                    }
                }
                Err(err) => return Err(LightningError::ConnectWebsocket(err.to_string())),
            }
        }

        debug!("LND invoice stream ended (next() returned None)");
        Ok(())
    }

    async fn handle_transaction_messages(
        &self,
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<(), LightningError> {
        while let Some(message) = ws_stream.next().await {
            match message {
                Ok(msg) => {
                    if msg.is_text() {
                        let text = msg.into_text().unwrap();
                        // [DEBUG-FLAKE] raw frame as received from LND, before any parsing/filtering.
                        debug!(%text, "LND transaction frame received");
                        // A failed deposit/withdrawal write propagates so the supervisor
                        // reconnects and re-syncs; parse errors are skipped inside.
                        self.process_transaction_message(&text).await?;
                    } else if msg.is_close() {
                        debug!(?msg, "LND transaction websocket closed by peer");
                        return Ok(());
                    } else {
                        // [DEBUG-FLAKE] ping/pong/binary — currently ignored silently.
                        trace!(?msg, "LND transaction non-text frame");
                    }
                }
                Err(err) => return Err(LightningError::ConnectWebsocket(err.to_string())),
            }
        }

        // [DEBUG-FLAKE] stream ended without a close frame — surfaces as a reconnect upstream.
        debug!("LND transaction stream ended (next() returned None)");
        Ok(())
    }

    async fn process_invoice_message(&self, text: &str) -> Result<(), LightningError> {
        let value: Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(err) => {
                error!(%err, %text, "Failed to parse invoice message");
                return Ok(());
            }
        };

        if let Some(event) = value.get("result") {
            match serde_json::from_value::<InvoiceResponse>(event.clone()) {
                Ok(invoice) => {
                    debug!(state = %invoice.state, "LND invoice event parsed");
                    if invoice.state.as_str() == "SETTLED" {
                        if let Err(err) = self.services.event.invoice_paid(invoice.into()).await {
                            return Err(LightningError::EventProcessing(err.to_string()));
                        }
                    }
                }
                Err(err) => {
                    error!(%err, %text, "Failed to parse SubscribeInvoices event");
                }
            }
        } else {
            // [DEBUG-FLAKE] frame with no "result" key (e.g. an LND {"error": ...}) was silently dropped.
            warn!(%text, "LND invoice frame without 'result' key; ignoring");
        }

        Ok(())
    }

    async fn process_transaction_message(&self, text: &str) -> Result<(), LightningError> {
        let value: Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(err) => {
                error!(%err, "Failed to parse transaction message");
                return Ok(());
            }
        };

        if let Some(event) = value.get("result") {
            match serde_json::from_value::<TransactionResponse>(event.clone()) {
                Ok(transaction) => {
                    let transaction: BtcTransaction = transaction.into();
                    self.handle_transaction(transaction).await?;
                }
                Err(err) => {
                    error!(%err, %text, "Failed to parse SubscribeTransactions event");
                }
            }
        } else {
            // [DEBUG-FLAKE] frame with no "result" key (e.g. an LND {"error": ...}) was silently dropped.
            warn!(%text, "LND transaction frame without 'result' key; ignoring");
        }

        Ok(())
    }

    async fn handle_transaction(&self, transaction: BtcTransaction) -> Result<(), LightningError> {
        // [DEBUG-FLAKE] full parsed tx: direction, confirmation height, and every output's
        // ownership/address/amount, so we can see if a deposit arrived unconfirmed-only or was
        // filtered out by is_ours.
        debug!(
            txid = %transaction.txid,
            block_height = ?transaction.block_height,
            is_outgoing = transaction.is_outgoing,
            n_outputs = transaction.outputs.len(),
            outputs = ?transaction.outputs,
            "Handling LND transaction"
        );
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
            // [DEBUG-FLAKE] the output we are about to project (relevant ones only). If an expected
            // deposit never logs here, it was filtered out above (is_ours == false).
            debug!(txid = %transaction.txid, ?output, is_outgoing = transaction.is_outgoing, "Projecting LND output");
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

            // Don't swallow: a failed write must not advance the cursor, or the deposit is
            // lost. Propagating exits the listener so the supervisor reconnects and `sync()`
            // reruns from the un-advanced cursor, reprocessing idempotently.
            result.map_err(|e| LightningError::EventProcessing(e.to_string()))?;
        }

        if let Some(block_height) = transaction.block_height.filter(|&h| h > 0) {
            self.services
                .system
                .set_onchain_cursor(OnchainSyncCursor::BlockHeight(block_height))
                .await
                .map_err(|e| LightningError::Listener(e.to_string()))?;
        }

        Ok(())
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
