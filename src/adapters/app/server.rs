use std::sync::Arc;

use axum::Router;
use tracing::{info, trace};

use crate::{
    adapters::{
        app::app_state::AppState, auth::jwt::JWTValidator, database::sqlx::SQLxClient,
        lightning::breez::BreezClient, logging::tracing::setup_tracing, rgb::rgblib::RGBLibClient,
    },
    application::{dtos::AppConfig, errors::WebServerError},
    domains::{lightning::api::http::LightningHandler, rgb::api::http::RGBHandler},
};

pub struct App {
    router: Router,
}

impl App {
    pub async fn new(config: AppConfig) -> Self {
        setup_tracing(config.logging.clone());
        trace!("Starting server");

        // Create adapters
        let db_client = SQLxClient::connect(config.database.clone()).await.unwrap();
        let rgb_client = RGBLibClient::new(config.rgb.clone()).await.unwrap();
        let lightning_client = BreezClient::new(config.lightning.clone()).await.unwrap();
        let jwt_validator = JWTValidator::new(config.auth.jwt.clone()).await.unwrap();

        // Create App state
        let state = AppState {
            auth_enabled: config.auth.enabled,
            jwt_validator: Arc::new(jwt_validator),
            lightning_client: Arc::new(lightning_client),
            rgb_client: Arc::new(rgb_client),
            db_client: Arc::new(db_client),
        };

        let router = Router::new()
            .nest("/rgb", RGBHandler::routes())
            .nest("/.well-known", LightningHandler::well_known_routes())
            .nest("/lightning", LightningHandler::routes())
            .with_state(Arc::new(state));

        Self { router }
    }

    pub async fn start(&self, addr: &str) -> Result<(), WebServerError> {
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
