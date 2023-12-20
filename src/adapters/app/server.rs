use std::sync::Arc;

use axum::Router;
use tracing::info;

use crate::{
    adapters::{
        app::app_state::AppState, auth::jwt::JWTValidator, lightning::breez::BreezClient,
        logging::tracing::setup_tracing, rgb::rgblib::RGBLibClient,
    },
    application::{dtos::AppConfig, errors::WebServerError},
    domains::{lightning::api::http::LightningHandler, rgb::api::http::RGBHandler},
};

pub struct App {
    state: AppState,
}

impl App {
    pub async fn new(config: AppConfig) -> Self {
        setup_tracing(config.logging.clone());
        info!(config = ?config, "Starting server");

        // Create adapters
        let rgb_client = RGBLibClient::new(config.rgb.clone()).await.unwrap();
        let lightning_client = BreezClient::new(config.lightning.clone()).await.unwrap();
        let jwt_validator = JWTValidator::new(config.auth.jwt.clone()).await.unwrap();

        // Create App state
        let state = AppState {
            auth_enabled: config.auth.enabled,
            jwt_validator: Arc::new(jwt_validator),
            lightning_client: Arc::new(lightning_client),
            rgb_client: Arc::new(rgb_client),
        };

        Self { state }
    }

    pub async fn start(&self, addr: &str) -> Result<(), WebServerError> {
        let router = Router::new()
            .nest("/rgb", RGBHandler::routes())
            .nest("/.well-known", LightningHandler::well_known_routes())
            .nest("/lightning", LightningHandler::routes())
            .with_state(Arc::new(self.state.clone()));

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| WebServerError::Listener(e.to_string()))?;

        info!(addr, "Listening on");

        axum::serve(listener, router)
            .await
            .map_err(|e| WebServerError::Serve(e.to_string()))?;

        Ok(())
    }
}
