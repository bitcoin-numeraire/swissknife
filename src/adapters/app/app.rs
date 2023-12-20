use std::{
    net::{AddrParseError, SocketAddr},
    sync::Arc,
};

use axum::{Extension, Router, Server};

use crate::{
    adapters::{
        app::app_state::AppState,
        auth::jwt::JWTValidator,
        lightning::{breez::BreezClient, DynLightningClient},
        logging::tracing::setup_tracing,
        rgb::{rgblib::RGBLibClient, DynRGBClient},
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

        // Create adapters
        let rgb_client = RGBLibClient::new(config.rgb.clone()).await.unwrap();
        let lightning_client = BreezClient::new(config.lightning.clone()).await.unwrap();
        let jwt_validator = JWTValidator::new(config.auth.jwt.clone()).await.unwrap();

        // Create App state
        let app_state = AppState {
            auth_enabled: config.auth.enabled,
            jwt_validator,
        };

        let router = Router::new()
            .nest(
                "/rgb",
                RGBHandler::routes(Arc::new(rgb_client) as DynRGBClient),
            )
            .nest("/.well-known", LightningHandler::well_known_routes())
            .nest(
                "/lightning",
                LightningHandler::routes(Arc::new(lightning_client) as DynLightningClient),
            )
            .layer(Extension(app_state));

        Self { router }
    }

    pub async fn start(&self, addr: &str) -> Result<(), WebServerError> {
        let socket_addr: SocketAddr = addr
            .parse()
            .map_err(|e: AddrParseError| WebServerError::Parse(e.to_string()))?;

        Server::bind(&socket_addr)
            .serve(self.router.clone().into_make_service())
            .await
            .map_err(|e| WebServerError::Listener(e.to_string()))?;

        Ok(())
    }
}
