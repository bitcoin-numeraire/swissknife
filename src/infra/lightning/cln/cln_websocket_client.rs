use std::sync::Arc;

use futures_util::{future::BoxFuture, FutureExt};
use rust_socketio::{
    asynchronous::{Client, ClientBuilder},
    Payload,
};
use tracing::{debug, error, warn};

use crate::{
    application::errors::LightningError,
    domains::lightning::services::LnEventsUseCases,
    infra::lightning::cln::cln_websocket_types::{CoinMovement, SendPayFailure, SendPaySuccess},
};

use super::ClnRestClientConfig;

pub async fn connect_websocket(
    config: &ClnRestClientConfig,
    ln_events: Arc<dyn LnEventsUseCases>,
) -> Result<Client, LightningError> {
    let client = ClientBuilder::new(config.endpoint.clone())
        .reconnect_on_disconnect(true)
        .opening_header("rune", config.rune.clone())
        .reconnect_delay(
            config.ws_min_reconnect_delay_delay,
            config.ws_max_reconnect_delay_delay,
        )
        .on("open", on_open)
        .on("close", on_close)
        .on("error", on_error)
        .on("message", {
            move |payload, socket: Client| on_message(ln_events.clone(), payload, socket)
        })
        .connect()
        .await
        .map_err(|e| LightningError::ConnectWebsocket(e.to_string()))?;

    Ok(client)
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
                    if let Some(event) = value.get("coin_movement") {
                        match serde_json::from_value::<CoinMovement>(event.clone()) {
                            Ok(coin_movement) if coin_movement.movement_type == "channel_mvt" => {
                                // It is not an invoice event, ignore
                                if !coin_movement.tags.iter().any(|e| e == "invoice") {
                                    continue;
                                }

                                // It is not a credit event, ignore
                                if coin_movement.credit_msat == 0 {
                                    continue;
                                }

                                if let Err(err) = ln_events.invoice_paid(coin_movement.into()).await
                                {
                                    warn!(%err, "Failed to process incoming payment");
                                }
                            }
                            Ok(_) => {}
                            Err(err) => {
                                error!(?err, "Failed to parse coin_movement event");
                            }
                        }
                    }

                    if let Some(event) = value.get("sendpay_success") {
                        match serde_json::from_value::<SendPaySuccess>(event.clone()) {
                            Ok(sendpay_success) => {
                                if sendpay_success.status != "complete" {
                                    warn!(
                                        payment_hash = sendpay_success.payment_hash,
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
                                        "Invalid payment status. Expected Failed."
                                    );
                                    return;
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
