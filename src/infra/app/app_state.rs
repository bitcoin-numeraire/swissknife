use std::sync::Arc;

use crate::{
    application::{
        dtos::AppConfig,
        errors::{ApplicationError, WebServerError},
    },
    domains::lightning::{
        adapters::LightningStore,
        usecases::{service::LightningPaymentsProcessor, LightningService, LightningUseCases},
    },
    infra::{
        auth::{jwt::JWTAuthenticator, Authenticator},
        database::sea_orm::SeaORMClient,
        lightning::breez::BreezListener,
    },
};
use humantime::parse_duration;
use tower_http::timeout::TimeoutLayer;
use tracing::warn;

#[derive(Clone)]
pub struct AppState {
    pub jwt_authenticator: Option<Arc<dyn Authenticator>>,
    pub lightning: Arc<dyn LightningUseCases>,
    pub timeout_layer: TimeoutLayer,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, ApplicationError> {
        let timeout_request = parse_duration(&config.web.request_timeout)
            .map_err(|e: humantime::DurationError| WebServerError::ParseConfig(e.to_string()))?;

        // Create infra
        let timeout_layer = TimeoutLayer::new(timeout_request);
        let db_conn = SeaORMClient::connect(config.database.clone()).await?;
        let jwt_authenticator = if config.auth.enabled {
            Some(
                Arc::new(JWTAuthenticator::new(config.auth.jwt.clone()).await?)
                    as Arc<dyn Authenticator>,
            )
        } else {
            warn!("Authentication disabled, all requests will be accepted as superuser");
            None
        };

        // Create adapters
        let store = Box::new(LightningStore::new(db_conn));
        let payments_processor = LightningPaymentsProcessor::new(store.clone());
        let listener = BreezListener::new(Arc::new(payments_processor));
        let lightning_client = config.lightning.get_client(Box::new(listener)).await?;

        // Create services
        let lightning = LightningService::new(
            store.clone(),
            lightning_client,
            config.lightning.domain,
            config.lightning.invoice_expiry,
            config.lightning.invoice_description,
        );

        // Create App state
        Ok(Self {
            jwt_authenticator,
            lightning: Arc::new(lightning),
            timeout_layer,
        })
    }
}
