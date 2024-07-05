use std::sync::Arc;

use axum::Router;
use std::future::Future;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use utoipa_scalar::{Scalar, Servable as ScalarServable};

use crate::{
    application::{docs::merged_openapi, entities::LnNodeClient, errors::WebServerError},
    domains::{invoices, lightning, payments, system, users, wallet},
    infra::app::AppState,
};

pub struct Server {
    router: Router,
}

impl Server {
    pub fn new(state: Arc<AppState>) -> Self {
        let router = Router::new()
            .nest("/api/system", system::api::router())
            .nest("/.well-known/lnurlp", lightning::api::well_known_router())
            .nest("/api/lnurlp", lightning::api::callback_router())
            .nest(
                "/api/lightning/addresses",
                lightning::api::ln_address_router(),
            )
            .nest("/api/invoices", invoices::api::router())
            .nest("/api/payments", payments::api::router())
            .nest("/api/wallet", wallet::api::router())
            .nest("/api/auth", users::api::auth_router())
            .merge(Scalar::with_url("/docs", merged_openapi()));

        let router = match state.ln_node_client {
            LnNodeClient::Breez(_) => {
                router.nest("/api/lightning/node", lightning::api::breez_node_router())
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
