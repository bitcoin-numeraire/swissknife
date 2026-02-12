use std::sync::Arc;

use crate::{
    application::{
        docs::merged_openapi,
        entities::{AppAdapters, AppServices},
        errors::WebServerError,
    },
    domains::{bitcoin, invoice, ln_address, lnurl, nostr, payment, system, user, wallet},
};
use axum::{routing::get, Router};
use std::future::Future;
use tokio::net::TcpListener;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use utoipa_scalar::{Scalar, Servable as ScalarServable};

pub struct Server {
    router: Router,
}

impl Server {
    pub fn new(adapters: AppAdapters, services: Arc<AppServices>, dashboard_dir: Option<&str>) -> Self {
        let router = Router::new()
            .nest("/.well-known", Self::well_known_router())
            .nest("/v1/system", system::router())
            .nest("/lnurlp", lnurl::router())
            .nest("/v1/invoices", invoice::router())
            .nest("/v1/payments", payment::router())
            .nest("/v1/me", wallet::user_router())
            .nest("/v1/wallets", wallet::router())
            .nest("/v1/auth", user::auth_router())
            .nest("/v1/api-keys", user::api_key_router())
            .nest("/v1/lightning-addresses", ln_address::router())
            .nest("/v1/bitcoin/addresses", bitcoin::router())
            .merge(Scalar::with_url("/docs", merged_openapi()));

        let router = match dashboard_dir {
            Some(dir) => router
                .fallback_service(ServeDir::new(dir).not_found_service(ServeFile::new(format!("{}/404.html", dir)))),
            None => router,
        };

        let router = router
            .layer(TraceLayer::new_for_http())
            .layer(adapters.timeout_layer)
            .layer(CorsLayer::permissive())
            .with_state(services);

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

    fn well_known_router() -> Router<Arc<AppServices>> {
        Router::new()
            .route("/lnurlp/{username}", get(lnurl::well_known))
            .route("/nostr.json", get(nostr::well_known_nostr))
    }
}
