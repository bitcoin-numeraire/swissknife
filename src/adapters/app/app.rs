use std::sync::Arc;

use axum::Router;
use tracing::{info, trace, warn};

use crate::{
    adapters::{
        app::AppState,
        auth::{jwt::JWTAuthenticator, Authenticator},
        database::sqlx::SQLxClient,
        lightning::breez::BreezClient,
        logging::tracing::setup_tracing,
        rgb::rgblib::RGBLibClient,
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
        let jwt_authenticator = if config.auth.enabled {
            Some(Arc::new(
                JWTAuthenticator::new(config.auth.jwt.clone())
                    .await
                    .unwrap(),
            ) as Arc<dyn Authenticator>)
        } else {
            warn!("Authentication disabled, all requests will be accepted as superuser");
            None
        };

        // TODO: Create services (use cases)

        // Create App state
        let state = AppState {
            jwt_authenticator,
            lightning_client: Arc::new(lightning_client),
            rgb_client: Arc::new(rgb_client),
            db_client: Arc::new(db_client),
        };

        // Create controllers (handlers)
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
