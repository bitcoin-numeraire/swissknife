use std::sync::Arc;

use crate::{
    adapters::{auth::Authenticator, rgb::RGBClient},
    application::errors::ApplicationError,
    domains::lightning::usecases::LightningUseCases,
};
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
    pub rgb_client: Arc<dyn RGBClient>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, ApplicationError> {
        // Create adapters
        let db_client = SQLxClient::connect(config.database.clone()).await?;
        let rgb_client = RGBLibClient::new(config.rgb.clone()).await?;
        let lightning_client = BreezClient::new(config.lightning.clone()).await?;
        let jwt_authenticator = if config.auth.enabled {
            Some(
                Arc::new(JWTAuthenticator::new(config.auth.jwt.clone()).await?)
                    as Arc<dyn Authenticator>,
            )
        } else {
            warn!("Authentication disabled, all requests will be accepted as superuser");
            None
        };

        // Create services (use cases)
        let lightning = LightningService::new(Box::new(db_client), Box::new(lightning_client));

        // Create App state
        Ok(Self {
            jwt_authenticator,
            lightning: Arc::new(lightning),
            rgb_client: Arc::new(rgb_client),
        })
    }
}
