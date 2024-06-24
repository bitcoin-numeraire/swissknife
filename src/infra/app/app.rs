use std::sync::Arc;

use axum::Router;
use std::future::Future;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::{
    application::{entities::LnNodeClient, errors::WebServerError},
    domains::{
        invoices::api::InvoiceHandler,
        lightning::api::{BreezNodeHandler, ClnNodeHandler, LnAddressHandler, LnURLpHandler},
        payments::api::PaymentHandler,
        wallet::api::WalletHandler,
    },
    infra::app::AppState,
};

pub struct App {
    router: Router,
}

impl App {
    pub fn new(state: Arc<AppState>) -> Self {
        let router = Router::new()
            .nest("/.well-known/lnurlp", LnURLpHandler::well_known_route())
            .nest("/api/lnurlp", LnURLpHandler::callback_route())
            .nest("/api/lightning/addresses", LnAddressHandler::routes())
            .nest(
                "/api/lightning/node",
                match state.ln_node_client {
                    LnNodeClient::Breez(_) => BreezNodeHandler::routes(),
                    LnNodeClient::ClnGrpc(_) => ClnNodeHandler::routes(),
                    LnNodeClient::ClnRest(_) => ClnNodeHandler::routes(),
                },
            )
            .nest("/api/invoices", InvoiceHandler::routes())
            .nest("/api/payments", PaymentHandler::routes())
            .nest("/api/wallet", WalletHandler::routes())
            .layer(TraceLayer::new_for_http())
            .layer(state.timeout_layer)
            .layer(CorsLayer::permissive())
            .with_state(state);

        Self { router }
    }

    pub async fn start(
        &self,
        addr: &str,
        shutdown_signal: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), WebServerError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| WebServerError::Listener(e.to_string()))?;

        info!(addr, "Listening on");

        axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| WebServerError::Serve(e.to_string()))?;

        Ok(())
    }
}
