use std::sync::Arc;

use crate::{
    adapters::{auth::Authenticator, lightning::breez::BreezListener, rgb::RGBClient},
    application::errors::{ApplicationError, WebServerError},
    domains::lightning::{
        store::sqlx::{SqlxLightningAddressRepository, SqlxLightningInvoiceRepository},
        usecases::{service::LightningPaymentsProcessor, LightningUseCases},
    },
};
use humantime::parse_duration;
use tower_http::timeout::TimeoutLayer;
use tracing::warn;

use crate::{
    adapters::{
        auth::jwt::JWTAuthenticator, database::sqlx::SQLxClient, lightning::breez::BreezClient,
        rgb::rgblib::RGBLibClient,
    },
    application::dtos::AppConfig,
    domains::lightning::usecases::LightningService,
};

#[derive(Clone)]
pub struct AppState {
    pub jwt_authenticator: Option<Arc<dyn Authenticator>>,
    pub lightning: Arc<dyn LightningUseCases>,
    pub rgb: Arc<dyn RGBClient>,
    pub timeout_layer: TimeoutLayer,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, ApplicationError> {
        let timeout_request = parse_duration(&config.web.request_timeout)
            .map_err(|e| WebServerError::ParseConfig(e.to_string()))?;

        // Create adapters
        let timeout_layer = TimeoutLayer::new(timeout_request);
        let db_client = SQLxClient::connect(config.database.clone()).await?;
        let rgb_client = RGBLibClient::new(config.rgb.clone()).await?;
        let jwt_authenticator = if config.auth.enabled {
            Some(
                Arc::new(JWTAuthenticator::new(config.auth.jwt.clone()).await?)
                    as Arc<dyn Authenticator>,
            )
        } else {
            warn!("Authentication disabled, all requests will be accepted as superuser");
            None
        };

        // Create repositories
        let lightning_address = Box::new(SqlxLightningAddressRepository::new(db_client.clone()));
        let lightning_invoice = Box::new(SqlxLightningInvoiceRepository::new(db_client.clone()));
        let payments_processor = LightningPaymentsProcessor::new(lightning_invoice.clone());

        // Create services
        let listener = BreezListener::new(Arc::new(payments_processor));
        let lightning_client =
            BreezClient::new(config.lightning.clone(), Box::new(listener)).await?;
        let lightning = LightningService::new(
            lightning_invoice.clone(),
            lightning_address,
            Box::new(lightning_client),
            config.lightning.domain,
        );
        // let rgb = RGBService::new(Box::new(rgb_client));

        // Create App state
        Ok(Self {
            jwt_authenticator,
            lightning: Arc::new(lightning),
            rgb: Arc::new(rgb_client),
            timeout_layer,
        })
    }
}
