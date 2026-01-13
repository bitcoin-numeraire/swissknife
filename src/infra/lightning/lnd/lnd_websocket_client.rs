use std::str::FromStr;
use std::{cmp::min, path::PathBuf, sync::Arc};

use futures_util::StreamExt;
use http::Uri;
use native_tls::{Certificate, TlsConnector};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::{fs, time::sleep};
use tokio_tungstenite::tungstenite::ClientRequestBuilder;
use tokio_tungstenite::{connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, warn};

use crate::domains::bitcoin::{BitcoinNetwork, BitcoinTransaction};
use crate::infra::lightning::lnd::lnd_types::{InvoiceResponse, Transaction};
use crate::{
    application::errors::LightningError,
    domains::{bitcoin::BitcoinEventsUseCases, ln_node::LnEventsUseCases},
};

use super::LndRestClientConfig;

pub async fn listen_invoices(
    config: LndRestClientConfig,
    macaroon: String,
    ln_events: Arc<dyn LnEventsUseCases>,
) -> Result<(), LightningError> {
    let max_reconnect_delay = config.ws_max_reconnect_delay;
    let mut reconnect_delay = config.ws_min_reconnect_delay;

    loop {
        let result = connect_and_handle(&macaroon, &config, ln_events.clone()).await;

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

pub async fn listen_transactions(
    config: LndRestClientConfig,
    macaroon: String,
    bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
    network: BitcoinNetwork,
) -> Result<(), LightningError> {
    let max_reconnect_delay = config.ws_max_reconnect_delay;
    let mut reconnect_delay = config.ws_min_reconnect_delay;

    loop {
        let result = connect_and_handle_transactions(&macaroon, &config, bitcoin_events.clone(), network).await;

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

async fn connect_and_handle(
    macaroon: &str,
    config: &LndRestClientConfig,
    ln_events: Arc<dyn LnEventsUseCases>,
) -> Result<(), LightningError> {
    let invoices_endpoint = format!("wss://{}/v1/invoices/subscribe", config.host);
    let uri = Uri::from_str(&invoices_endpoint).map_err(|e| LightningError::ParseConfig(e.to_string()))?;
    let builder = ClientRequestBuilder::new(uri).with_header("Grpc-Metadata-Macaroon", macaroon);

    let tls_connector = create_tls_connector(config).await?;

    let (ws_stream, _) = connect_async_tls_with_config(builder, None, false, tls_connector)
        .await
        .map_err(|e| LightningError::ConnectWebsocket(e.to_string()))?;

    debug!("Connected to LND WebSocket server");

    handle_messages(ws_stream, ln_events).await;

    debug!("Disconnected from LND WebSocket server");

    Ok(())
}

async fn connect_and_handle_transactions(
    macaroon: &str,
    config: &LndRestClientConfig,
    bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
    network: BitcoinNetwork,
) -> Result<(), LightningError> {
    let endpoint = format!("wss://{}/v1/transactions/subscribe", config.host);
    let uri = Uri::from_str(&endpoint).map_err(|e| LightningError::ParseConfig(e.to_string()))?;
    let builder = ClientRequestBuilder::new(uri).with_header("Grpc-Metadata-Macaroon", macaroon);

    let tls_connector = create_tls_connector(config).await?;

    let (ws_stream, _) = connect_async_tls_with_config(builder, None, false, tls_connector)
        .await
        .map_err(|e| LightningError::ConnectWebsocket(e.to_string()))?;

    debug!("Connected to LND WebSocket transaction server");

    handle_transaction_messages(ws_stream, bitcoin_events, network).await;

    debug!("Disconnected from LND WebSocket transaction server");

    Ok(())
}

async fn create_tls_connector(config: &LndRestClientConfig) -> Result<Option<Connector>, LightningError> {
    if let Some(ca_cert_path) = &config.ca_cert_path {
        let ca_certificate = read_ca(ca_cert_path)
            .await
            .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;
        let tls_connector = TlsConnector::builder()
            .add_root_certificate(ca_certificate)
            .danger_accept_invalid_hostnames(config.accept_invalid_hostnames)
            .build()
            .map_err(|e| LightningError::TLSConfig(e.to_string()))?;
        Ok(Some(Connector::NativeTls(tls_connector)))
    } else if config.accept_invalid_certs || config.accept_invalid_hostnames {
        let tls_connector = TlsConnector::builder()
            .danger_accept_invalid_hostnames(config.accept_invalid_hostnames)
            .build()
            .map_err(|e| LightningError::TLSConfig(e.to_string()))?;
        Ok(Some(Connector::NativeTls(tls_connector)))
    } else {
        Ok(None)
    }
}

async fn read_ca(path: &str) -> anyhow::Result<Certificate> {
    let ca_file = fs::read(PathBuf::from(path)).await?;
    let ca_certificate = Certificate::from_pem(&ca_file)?;
    Ok(ca_certificate)
}

async fn handle_messages(
    mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ln_events: Arc<dyn LnEventsUseCases>,
) {
    while let Some(message) = ws_stream.next().await {
        match message {
            Ok(msg) => {
                if msg.is_text() {
                    let text = msg.into_text().unwrap();
                    if let Err(e) = process_message(&text, ln_events.clone()).await {
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

async fn handle_transaction_messages(
    mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
    network: BitcoinNetwork,
) {
    while let Some(message) = ws_stream.next().await {
        match message {
            Ok(msg) => {
                if msg.is_text() {
                    let text = msg.into_text().unwrap();
                    if let Err(e) = process_transaction_message(&text, bitcoin_events.clone(), network).await {
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

async fn process_message(text: &str, ln_events: Arc<dyn LnEventsUseCases>) -> anyhow::Result<()> {
    let value: Value = serde_json::from_str(text)?;

    if let Some(event) = value.get("result") {
        match serde_json::from_value::<InvoiceResponse>(event.clone()) {
            Ok(invoice) => {
                if invoice.state.as_str() == "SETTLED" {
                    if let Err(err) = ln_events.invoice_paid(invoice.into()).await {
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

async fn process_transaction_message(
    text: &str,
    bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
    network: BitcoinNetwork,
) -> anyhow::Result<()> {
    let value: Value = serde_json::from_str(text)?;

    if let Some(event) = value.get("result") {
        match serde_json::from_value::<Transaction>(event.clone()) {
            Ok(transaction) => {
                let transaction: BitcoinTransaction = transaction.into();
                for output in transaction.outputs.iter() {
                    let output_event = transaction.output_event(output);
                    let result = if output.is_ours {
                        bitcoin_events.onchain_deposit(output_event, network).await
                    } else {
                        bitcoin_events.onchain_withdrawal(output_event, network).await
                    };

                    if let Err(err) = result {
                        warn!(%err, "Failed to process onchain transaction");
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
