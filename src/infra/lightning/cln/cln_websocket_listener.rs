use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures_util::{future::BoxFuture, FutureExt};
use native_tls::{Certificate, TlsConnector};
use rust_socketio::{
    asynchronous::{Client, ClientBuilder, ReconnectSettings},
    Payload, TransportType,
};
use tokio::{
    fs,
    sync::{self, mpsc},
    time::sleep,
};
use tracing::{debug, error, info, warn};

use crate::{
    application::{
        composition::{AppServices, Currency},
        errors::{ApplicationError, LightningError},
    },
    domains::bitcoin::{BitcoinWallet, OnchainSyncCursor, OnchainTransaction},
    infra::lightning::EventsListener,
};

use super::cln_websocket_types::{InvoicePayment, SendPayFailure, SendPaySuccess};
use super::ClnRestClientConfig;

pub struct ClnWebsocketListener {
    config: ClnRestClientConfig,
    services: Arc<AppServices>,
    wallet: Arc<dyn BitcoinWallet>,
}

impl ClnWebsocketListener {
    pub async fn new(
        config: ClnRestClientConfig,
        services: Arc<AppServices>,
        wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        Ok(Self {
            config,
            services,
            wallet,
        })
    }

    async fn client_builder(&self) -> Result<ClientBuilder, LightningError> {
        let config = &self.config;
        let reconnect_delay_min = config.ws_min_reconnect_delay;
        let reconnect_delay_max = config.ws_max_reconnect_delay;
        let mut client_builder = ClientBuilder::new(config.endpoint.clone())
            .transport_type(TransportType::Websocket)
            .reconnect_on_disconnect(true)
            .opening_header("rune", config.rune.clone())
            .reconnect_delay(
                config.ws_min_reconnect_delay.as_secs(),
                config.ws_max_reconnect_delay.as_secs(),
            )
            .on_reconnect({
                let services = self.services.clone();
                move || {
                    let services = services.clone();
                    async move {
                        ClnWebsocketListener::resync(
                            &services,
                            "Core Lightning websocket reconnect",
                            reconnect_delay_min,
                            reconnect_delay_max,
                        )
                        .await;
                        ReconnectSettings::new()
                    }
                    .boxed()
                }
            })
            .on("open", on_open)
            .on("close", on_close)
            .on("error", on_error);

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

        Ok(client_builder)
    }

    fn on_message(
        services: Arc<AppServices>,
        wallet: Arc<dyn BitcoinWallet>,
        failure_tx: mpsc::Sender<LightningError>,
        cursor: Arc<tokio::sync::Mutex<Option<OnchainSyncCursor>>>,
        payload: Payload,
        client: Client,
    ) -> BoxFuture<'static, ()> {
        async move {
            match payload {
                Payload::Text(values) => {
                    for value in values {
                        if let Some(event) = value.get("invoice_payment") {
                            match serde_json::from_value::<InvoicePayment>(event.clone()) {
                                Ok(invoice_payment) => {
                                    if let Err(err) = services.event.invoice_paid(invoice_payment.into()).await {
                                        Self::stop_after_projection_error(
                                            &failure_tx,
                                            &client,
                                            "incoming_payment",
                                            err,
                                        )
                                        .await;
                                        return;
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
                                        Self::stop_after_projection_error(
                                            &failure_tx,
                                            &client,
                                            "outgoing_payment",
                                            err,
                                        )
                                        .await;
                                        return;
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
                                        Self::stop_after_projection_error(&failure_tx, &client, "failed_payment", err)
                                            .await;
                                        return;
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
                                    let mut processed_all = true;
                                    for transaction in batch.events {
                                        let result = match transaction {
                                            OnchainTransaction::Deposit(output) => {
                                                services.event.onchain_deposit(output.into(), currency.clone()).await
                                            }
                                            OnchainTransaction::Withdrawal(event) => {
                                                services.event.onchain_withdrawal(event).await
                                            }
                                        };

                                        // Don't advance the cursor past a write we couldn't persist, or the
                                        // deposit is lost. Unlike a busy chain, no further `coin_movement` is
                                        // guaranteed to arrive to re-trigger this sync (e.g. the deposit is the
                                        // last on-chain activity), so signal the supervisor to reconnect just
                                        // like the off-chain events above: re-entering listen() re-runs
                                        // bitcoin.sync() from the un-advanced cursor and reprocesses idempotently.
                                        if let Err(err) = result {
                                            error!(%err, "Failed to process onchain transaction; cursor not advanced");
                                            processed_all = false;
                                            Self::stop_after_projection_error(&failure_tx, &client, "onchain", err)
                                                .await;
                                            break;
                                        }
                                    }

                                    if processed_all {
                                        if let Some(next_cursor) = batch.next_cursor {
                                            match services.system.set_onchain_cursor(next_cursor.clone()).await {
                                                Ok(()) => *cursor_guard = Some(next_cursor),
                                                Err(err) => warn!(%err, "Failed to persist chainmoves cursor"),
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!(%err, "Failed to synchronize onchain transactions; reconnecting listener");
                                    Self::stop_after_projection_error(&failure_tx, &client, "onchain_sync", err.into())
                                        .await;
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

    async fn stop_after_projection_error(
        failure_tx: &mpsc::Sender<LightningError>,
        client: &Client,
        event_type: &'static str,
        err: ApplicationError,
    ) {
        Self::signal_listener_failure(failure_tx, event_type, err);

        if let Err(err) = client.disconnect().await {
            debug!(%err, event_type, "Failed to disconnect Core Lightning websocket after projection error");
        }
    }

    fn signal_listener_failure(
        failure_tx: &mpsc::Sender<LightningError>,
        event_type: &'static str,
        err: ApplicationError,
    ) {
        let err = err.to_string();
        error!(%err, event_type, "Failed to process Lightning event; reconnecting listener");

        if let Err(send_err) = failure_tx.try_send(LightningError::EventProcessing(err)) {
            debug!(
                ?send_err,
                event_type, "Core Lightning websocket listener failure already signaled"
            );
        }
    }

    /// Replay off-chain (invoice/payment) AND on-chain (deposit/withdrawal) state, retrying with
    /// backoff until it succeeds. A transport-level socket.io reconnect does not re-enter listen(),
    /// so bitcoin.sync() must run here too; otherwise a deposit whose coin_movement arrived during
    /// the disconnect stays uncredited until the next chain event.
    async fn resync(services: &AppServices, context: &'static str, min_delay: Duration, max_delay: Duration) {
        let min_delay = if min_delay.is_zero() {
            Duration::from_millis(100)
        } else {
            min_delay
        };
        let max_delay = max_delay.max(min_delay);
        let mut backoff = min_delay;

        loop {
            match tokio::try_join!(
                services.invoice.sync(),
                services.payment.sync(),
                services.bitcoin.sync()
            ) {
                Ok((synced_invoices, synced_payments, synced_onchain)) => {
                    debug!(
                        context,
                        synced_invoices, synced_payments, synced_onchain, "State replayed after reconnect"
                    );
                    return;
                }
                Err(err) => {
                    error!(%err, context, ?backoff, "State replay failed; retrying before reconnect");
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(max_delay);
                }
            }
        }
    }
}

#[async_trait]
impl EventsListener for ClnWebsocketListener {
    async fn listen(&self) -> Result<(), LightningError> {
        let mut client_builder = self.client_builder().await?;

        self.services
            .bitcoin
            .sync()
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        let initial_cursor = self
            .services
            .system
            .get_onchain_cursor()
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;
        let cursor = Arc::new(sync::Mutex::new(initial_cursor));

        let wallet = self.wallet.clone();
        let services = self.services.clone();
        let (failure_tx, mut failure_rx) = mpsc::channel(1);

        client_builder = client_builder.on("message", move |payload, client| {
            Self::on_message(
                services.clone(),
                wallet.clone(),
                failure_tx.clone(),
                cursor.clone(),
                payload,
                client,
            )
        });

        // Hold the connected client: rust_socketio reconnects internally
        // (reconnect_on_disconnect) for transport failures. Projection failures are
        // signaled from callbacks so the supervisor can apply its backoff and replay sync.
        let _client = client_builder
            .connect()
            .await
            .map_err(|e| LightningError::ConnectWebsocket(e.to_string()))?;

        Err(failure_rx
            .recv()
            .await
            .unwrap_or_else(|| LightningError::Listener("Core Lightning websocket listener stopped".to_string())))
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

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use tokio::sync::mpsc;

    use crate::application::{
        composition::MockAppServicesBuilder,
        errors::{DataError, DatabaseError, LightningError},
    };

    use super::ClnWebsocketListener;

    #[test]
    fn offchain_projection_error_signals_listener_failure() {
        let (failure_tx, mut failure_rx) = mpsc::channel(1);

        ClnWebsocketListener::signal_listener_failure(
            &failure_tx,
            "incoming_payment",
            DatabaseError::Update("database is locked".to_string()).into(),
        );

        let err = failure_rx.try_recv().expect("listener failure");
        assert!(matches!(
            err,
            LightningError::EventProcessing(message) if message.contains("database is locked")
        ));
    }

    #[test]
    fn repeated_offchain_projection_error_is_dropped_after_failure_is_signaled() {
        let (failure_tx, mut failure_rx) = mpsc::channel(1);

        ClnWebsocketListener::signal_listener_failure(
            &failure_tx,
            "incoming_payment",
            DatabaseError::Update("database is locked".to_string()).into(),
        );
        ClnWebsocketListener::signal_listener_failure(
            &failure_tx,
            "incoming_payment",
            DataError::NotFound("Lightning invoice not found.".to_string()).into(),
        );

        assert!(failure_rx.try_recv().is_ok());
        assert!(failure_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn reconnect_replay_retries_until_sync_succeeds() {
        let invoice_attempts = Arc::new(AtomicUsize::new(0));
        let mut builder = MockAppServicesBuilder::new();

        builder.invoice.expect_sync().times(2).returning({
            let invoice_attempts = invoice_attempts.clone();
            move || {
                if invoice_attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(DatabaseError::Update("transient failure".to_string()).into())
                } else {
                    Ok(1)
                }
            }
        });
        builder.payment.expect_sync().returning(|| Ok(1));
        // The reconnect replay also re-syncs on-chain state (bitcoin.sync).
        builder.bitcoin.expect_sync().returning(|| Ok(0));
        let services = builder.build();

        ClnWebsocketListener::resync(
            &services,
            "test reconnect",
            Duration::from_millis(1),
            Duration::from_millis(1),
        )
        .await;

        assert_eq!(invoice_attempts.load(Ordering::SeqCst), 2);
    }
}
