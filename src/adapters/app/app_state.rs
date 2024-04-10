use std::sync::Arc;

use crate::{
    adapters::{
        auth::{jwt::JWTAuthenticator, Authenticator},
        database::sqlx::PgClient,
        lightning::breez::{BreezClient, BreezListener},
        rgb::{rgblib::RGBLibClient, RGBClient},
    },
    application::{
        dtos::AppConfig,
        errors::{ApplicationError, WebServerError},
    },
    domains::lightning::{
        store::LightningStore,
        store::PgLightningAddressRepository,
        store::PgLightningInvoiceRepository,
        store::PgLightningPaymentRepository,
        usecases::{service::LightningPaymentsProcessor, LightningService, LightningUseCases},
    },
};
use humantime::parse_duration;
use tower_http::timeout::TimeoutLayer;
use tracing::warn;

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
        let db_client = PgClient::connect(config.database.clone()).await?;
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

        // Create repositories and stores
        let store = LightningStore::new(
            Arc::new(PgLightningInvoiceRepository::new(db_client.clone())),
            Arc::new(PgLightningAddressRepository::new(db_client.clone())),
            Arc::new(PgLightningPaymentRepository::new(db_client.clone())),
        );

        // Create services
        let payments_processor = LightningPaymentsProcessor::new(store.clone());
        let listener = BreezListener::new(Arc::new(payments_processor));
        let lightning_client =
            BreezClient::new(config.lightning.clone(), Box::new(listener)).await?;
        let lightning = LightningService::new(
            store.clone(),
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
