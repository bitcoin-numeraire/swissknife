use std::sync::Arc;

use axum::Router;

use tokio::{
    net::TcpListener,
    signal::{
        ctrl_c,
        unix::{signal, SignalKind},
    },
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, error, info, trace};

use crate::{
    application::errors::WebServerError,
    domains::lightning::api::http::{
        LNURLpHandler, LightningAddressHandler, LightningNodeHandler, LightningWalletHandler,
    },
    infra::app::AppState,
};

pub struct App {
    router: Router,
}

impl App {
    pub fn new(state: AppState) -> Self {
        trace!("Initializing app");

        let router = Router::new()
            .nest("/lnurlp", LNURLpHandler::routes())
            .nest(
                "/api/lightning/addresses",
                LightningAddressHandler::routes(),
            )
            .nest("/api/lightning/wallet", LightningWalletHandler::routes())
            .nest("/api/lightning/node", LightningNodeHandler::routes())
            .layer(TraceLayer::new_for_http())
            .layer(state.timeout_layer)
            .layer(CorsLayer::permissive())
            .with_state(Arc::new(state));

        debug!("App initialized successfully");
        Self { router }
    }

    pub async fn start(&self, addr: &str) -> Result<(), WebServerError> {
        trace!("Starting app");

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| WebServerError::Listener(e.to_string()))?;

        info!(addr, "Listening on");

        axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(Self::shutdown_signal())
            .await
            .map_err(|e| WebServerError::Serve(e.to_string()))?;

        Ok(())
    }

    async fn shutdown_signal() {
        let ctrl_c = async {
            if let Err(e) = ctrl_c().await {
                error!(error = ?e, "Failed to install Ctrl+C handler");
            }
            info!("Received Ctrl+C signal. Shutting down gracefully");
        };

        #[cfg(unix)]
        let terminate = async {
            match signal(SignalKind::terminate()) {
                Ok(mut stream) => {
                    stream.recv().await;
                    info!("Received SIGTERM. Shutting down gracefully");
                }
                Err(e) => error!(error = ?e, "Failed to install SIGTERM handler"),
            }
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
    }
}
