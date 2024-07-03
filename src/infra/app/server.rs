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
        lightning::api::{BreezNodeHandler, LnAddressHandler, LnURLpHandler},
        payments::api::PaymentHandler,
        system::api::SystemHandler,
        users::api::UserHandler,
        wallet::api::WalletHandler,
    },
    infra::app::AppState,
};

pub struct Server {
    router: Router,
}

impl Server {
    pub fn new(state: Arc<AppState>) -> Self {
        let router = Router::new()
            .nest("/api/system", SystemHandler::routes())
            .nest("/.well-known/lnurlp", LnURLpHandler::well_known_route())
            .nest("/api/lnurlp", LnURLpHandler::callback_route())
            .nest("/api/lightning/addresses", LnAddressHandler::routes())
            .nest("/api/invoices", InvoiceHandler::routes())
            .nest("/api/payments", PaymentHandler::routes())
            .nest("/api/wallet", WalletHandler::routes())
            .nest("/api/users", UserHandler::routes());

        let router = match state.ln_node_client {
            LnNodeClient::Breez(_) => {
                router.nest("/api/lightning/node", BreezNodeHandler::routes())
            }
            _ => router,
        };

        let router = router
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
