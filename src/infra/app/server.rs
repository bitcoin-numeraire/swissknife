use std::sync::Arc;

use axum::Router;
use std::future::Future;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use utoipa_scalar::{Scalar, Servable as ScalarServable};

use crate::{
    application::{docs::merged_openapi, entities::LnNodeClient, errors::WebServerError},
    domains::{invoice, ln_address, ln_node, lnurl, payment, system, user, wallet},
    infra::app::AppState,
};

pub struct Server {
    router: Router,
}

impl Server {
    pub fn new(state: Arc<AppState>) -> Self {
        let router = Router::new()
            .nest("/api/system", system::router())
            .nest("/.well-known/lnurlp", lnurl::well_known_router())
            .nest("/api/lnurlp", lnurl::callback_router())
            .nest("/api/lightning/addresses", ln_address::router())
            .nest("/api/invoices", invoice::router())
            .nest("/api/payments", payment::router())
            .nest("/api/wallet", wallet::router())
            .nest("/api/auth", user::auth_router())
            .merge(Scalar::with_url("/docs", merged_openapi()));

        let router = match state.ln_node_client {
            LnNodeClient::Breez(_) => {
                router.nest("/api/lightning/node", ln_node::breez_node_router())
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
