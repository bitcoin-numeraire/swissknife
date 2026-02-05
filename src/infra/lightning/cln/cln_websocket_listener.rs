use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures_util::{future::BoxFuture, FutureExt};
use native_tls::{Certificate, TlsConnector};
use rust_socketio::{
    asynchronous::{Client, ClientBuilder},
    Payload, TransportType,
};
use tokio::fs;
use tracing::{error, info, warn};

use crate::{
    application::{
        entities::{AppServices, Currency},
        errors::LightningError,
    },
    domains::bitcoin::{BitcoinWallet, OnchainSyncCursor, OnchainTransaction},
    infra::lightning::EventsListener,
};

use super::cln_websocket_types::{InvoicePayment, SendPayFailure, SendPaySuccess};
use super::ClnRestClientConfig;

pub struct ClnWebsocketListener {
    client_builder: Mutex<Option<ClientBuilder>>,
}

impl ClnWebsocketListener {
    pub async fn new(
        config: ClnRestClientConfig,
        services: Arc<AppServices>,
        wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        let initial_cursor = services
            .system
            .get_onchain_cursor()
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;
        let cursor = Arc::new(tokio::sync::Mutex::new(initial_cursor));

        let mut client_builder = ClientBuilder::new(config.endpoint.clone())
            .transport_type(TransportType::Websocket)
            .reconnect_on_disconnect(true)
            .opening_header("rune", config.rune.clone())
            .reconnect_delay(
                config.ws_min_reconnect_delay.as_secs(),
                config.ws_max_reconnect_delay.as_secs(),
            )
            .on("open", on_open)
            .on("close", on_close)
            .on("error", on_error)
            .on("message", move |payload, _: Client| {
                Self::on_message(services.clone(), wallet.clone(), cursor.clone(), payload)
            });

        if let Some(ca_cert_path) = &config.ca_cert_path {
            let ca_certificate = read_ca(ca_cert_path)
                .await
                .map_err(|e| LightningError::ReadCertificates(e.to_string()))?;
            let tls_connector = TlsConnector::builder()
                .add_root_certificate(ca_certificate)
                .danger_accept_invalid_hostnames(config.accept_invalid_hostnames)
                .build()
                .map_err(|e| LightningError::TLSConfig(e.to_string()))?;
            client_builder = client_builder.tls_config(tls_connector.clone());
        }

        if config.accept_invalid_certs {
            let tls_connector = TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| LightningError::TLSConfig(e.to_string()))?;

            client_builder = client_builder.tls_config(tls_connector);
        }

        Ok(Self {
            client_builder: Mutex::new(Some(client_builder)),
        })
    }

    fn on_message(
        services: Arc<AppServices>,
        wallet: Arc<dyn BitcoinWallet>,
        cursor: Arc<tokio::sync::Mutex<Option<OnchainSyncCursor>>>,
        payload: Payload,
    ) -> BoxFuture<'static, ()> {
        async move {
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        if let Some(event) = value.get("invoice_payment") {
                            match serde_json::from_value::<InvoicePayment>(event.clone()) {
                                Ok(invoice_payment) => {
                                    if let Err(err) = services.event.invoice_paid(invoice_payment.into()).await {
                                        warn!(%err, "Failed to process incoming payment");
                                    }
                                }
                                Err(err) => {
                                    warn!(
                                        ?err,
                                        "Failed to parse invoice_payment event. Most likely an external payment"
                                    );
                                }
                            }
                        }

                        if let Some(event) = value.get("sendpay_success") {
                            match serde_json::from_value::<SendPaySuccess>(event.clone()) {
                                Ok(sendpay_success) => {
                                    if sendpay_success.status != "complete" {
                                        warn!(
                                            payment_hash = sendpay_success.payment_hash,
                                            status = sendpay_success.status,
                                            "Invalid payment status. Expected Complete."
                                        );
                                        return;
                                    }

                                    if let Err(err) = services.event.outgoing_payment(sendpay_success.into()).await {
                                        warn!(%err, "Failed to process outgoing payment");
                                    }
                                }
                                Err(err) => {
                                    error!(?err, "Failed to parse sendpay_success event");
                                }
                            }
                        }

                        if let Some(event) = value.get("sendpay_failure") {
                            match serde_json::from_value::<SendPayFailure>(event.clone()) {
                                Ok(sendpay_failure) => {
                                    if sendpay_failure.data.status != "failed" {
                                        warn!(
                                            payment_hash = sendpay_failure.data.payment_hash,
                                            status = sendpay_failure.data.status,
                                            "Invalid payment status. Expected Failed."
                                        );
                                        // We must accept the payment as failed until this is fixed: https://github.com/ElementsProject/lightning/issues/7561
                                        // return;
                                    }

                                    if let Err(err) = services.event.failed_payment(sendpay_failure.into()).await {
                                        warn!(%err, "Failed to process failed outgoing payment");
                                    }
                                }
                                Err(err) => {
                                    error!(?err, "Failed to parse sendpay_failure event");
                                }
                            }
                        }

                        if let Some(event) = value.get("coin_movement") {
                            if let Some(movement_type) = event.get("type").and_then(|t| t.as_str()) {
                                if movement_type != "chain_mvt" {
                                    continue;
                                }
                            } else {
                                continue;
                            }

                            let mut cursor_guard = cursor.lock().await;
                            let currency: Currency = wallet.network().into();

                            match wallet.synchronize(cursor_guard.clone()).await {
                                Ok(batch) => {
                                    for transaction in batch.events {
                                        match transaction {
                                            OnchainTransaction::Deposit(output) => {
                                                if let Err(err) = services
                                                    .event
                                                    .onchain_deposit(output.into(), currency.clone())
                                                    .await
                                                {
                                                    error!(%err, "Failed to process onchain deposit");
                                                }
                                            }
                                            OnchainTransaction::Withdrawal(event) => {
                                                if let Err(err) = services.event.onchain_withdrawal(event).await {
                                                    error!(%err, "Failed to process onchain withdrawal");
                                                }
                                            }
                                        }
                                    }

                                    if let Some(next_cursor) = batch.next_cursor {
                                        if let Err(err) = services.system.set_onchain_cursor(next_cursor.clone()).await
                                        {
                                            warn!(%err, "Failed to persist chainmoves cursor");
                                        }
                                        *cursor_guard = Some(next_cursor);
                                    }
                                }
                                Err(err) => {
                                    error!(%err, "Failed to synchronize onchain transactions");
                                }
                            }
                        }
                    }
                }
                _ => error!(?payload, "Non supported payload type"),
            }
        }
        .boxed()
    }
}

#[async_trait]
impl EventsListener for ClnWebsocketListener {
    async fn listen(&self) -> Result<(), LightningError> {
        let client_builder = self
            .client_builder
            .lock()
            .map_err(|_| LightningError::Listener("Failed to lock client builder".to_string()))?
            .take()
            .ok_or_else(|| LightningError::Listener("Listener already started".to_string()))?;

        let _client = client_builder
            .connect()
            .await
            .map_err(|e| LightningError::ConnectWebsocket(e.to_string()))?;

        Ok(())
    }
}

async fn read_ca(path: &str) -> anyhow::Result<Certificate> {
    let ca_file = fs::read(PathBuf::from(path)).await?;
    let ca_certificate = Certificate::from_pem(&ca_file)?;

    Ok(ca_certificate)
}

fn on_open(_: Payload, _: Client) -> BoxFuture<'static, ()> {
    async move {
        info!("Connected to Core Lightning websocket server");
    }
    .boxed()
}

fn on_close(_: Payload, _: Client) -> BoxFuture<'static, ()> {
    async move {
        info!("Disconnected from Core Lightning websocket server");
    }
    .boxed()
}

fn on_error(err: Payload, _: Client) -> BoxFuture<'static, ()> {
    async move {
        error!(?err, "Error from Core Lightning websocket server ");
    }
    .boxed()
}
