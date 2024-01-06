use std::sync::Arc;

use axum::Router;
use tracing::{info, trace};

use crate::{
    adapters::app::AppState,
    application::errors::WebServerError,
    domains::{lightning::api::http::LightningHandler, rgb::api::http::RGBHandler},
};

pub struct App {
    router: Router,
}

impl App {
    pub fn new(state: AppState) -> Self {
        trace!("Initializing app");

        let router = Router::new()
            .nest("/rgb", RGBHandler::routes())
            .nest("/.well-known/lnurlp", LightningHandler::well_known_routes())
            .nest("/lightning/addresses", LightningHandler::addresses_routes())
            .nest("/lightning/node", LightningHandler::node_routes())
            .with_state(Arc::new(state));

        trace!("App initialised successfully");

        Self { router }
    }

    pub async fn start(&self, addr: &str) -> Result<(), WebServerError> {
        trace!("Starting app");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| WebServerError::Listener(e.to_string()))?;

        info!(addr, "Listening on");

        axum::serve(listener, self.router.clone())
            .await
            .map_err(|e| WebServerError::Serve(e.to_string()))?;

        Ok(())
    }
}
