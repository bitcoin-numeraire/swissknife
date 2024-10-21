use std::{path::PathBuf, sync::Arc};

use futures_util::{future::BoxFuture, FutureExt};
use native_tls::{Certificate, TlsConnector};
use rust_socketio::{
    asynchronous::{Client, ClientBuilder},
    Payload,
};
use tokio::fs;
use tracing::{debug, error, warn};

use crate::{
    application::errors::LightningError,
    domains::ln_node::LnEventsUseCases,
    infra::lightning::cln::cln_websocket_types::{InvoicePayment, SendPayFailure, SendPaySuccess},
};

use super::ClnRestClientConfig;

pub async fn connect_websocket(
    config: &ClnRestClientConfig,
    ln_events: Arc<dyn LnEventsUseCases>,
) -> Result<Client, LightningError> {
    let mut client_builder = ClientBuilder::new(config.endpoint.clone())
        .reconnect_on_disconnect(true)
        .opening_header("rune", config.rune.clone())
        .reconnect_delay(
            config.ws_min_reconnect_delay.as_secs(),
            config.ws_max_reconnect_delay.as_secs(),
        )
        .on("open", on_open)
        .on("close", on_close)
        .on("error", on_error)
        .on("message", {
            move |payload, socket: Client| on_message(ln_events.clone(), payload, socket)
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

    let client = client_builder
        .connect()
        .await
        .map_err(|e| LightningError::ConnectWebsocket(e.to_string()))?;

    Ok(client)
}

async fn read_ca(path: &str) -> anyhow::Result<Certificate> {
    let ca_file = fs::read(PathBuf::from(path)).await?;
    let ca_certificate = Certificate::from_pem(&ca_file)?;

    Ok(ca_certificate)
}

fn on_open(_: Payload, _: Client) -> BoxFuture<'static, ()> {
    async move {
        debug!("Connected to Core Lightning websocket server");
    }
    .boxed()
}

fn on_close(_: Payload, _: Client) -> BoxFuture<'static, ()> {
    async move {
        debug!("Disconnected from Core Lightning websocket server");
    }
    .boxed()
}

fn on_error(err: Payload, _: Client) -> BoxFuture<'static, ()> {
    async move {
        error!(?err, "Error from Core Lightning websocket server ");
    }
    .boxed()
}

fn on_message(
    ln_events: Arc<dyn LnEventsUseCases>,
    payload: Payload,
    _: Client,
) -> BoxFuture<'static, ()> {
    async move {
        match payload {
            Payload::Text(values) => {
                for value in values {
                    if let Some(event) = value.get("invoice_payment") {
                        match serde_json::from_value::<InvoicePayment>(event.clone()) {
                            Ok(invoice_payment) => {
                                if let Err(err) =
                                    ln_events.invoice_paid(invoice_payment.into()).await
                                {
                                    warn!(%err, "Failed to process incoming payment");
                                }
                            }
                            Err(err) => {
                                warn!(?err, "Failed to parse invoice_payment event. Most likely an external payment");
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

                                if let Err(err) =
                                    ln_events.outgoing_payment(sendpay_success.into()).await
                                {
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

                                if let Err(err) =
                                    ln_events.failed_payment(sendpay_failure.into()).await
                                {
                                    warn!(%err, "Failed to process failed outgoing payment");
                                }
                            }
                            Err(err) => {
                                error!(?err, "Failed to parse sendpay_failure event");
                            }
                        }
                    }
                }
            }
            _ => error!(
                ?payload,
                "Non supported payload type from Core Lightning websocket server"
            ),
        }
    }
    .boxed()
}
