use std::sync::Arc;

use crate::{
    application::{
        dtos::{AppConfig, LightningProvider},
        entities::{AppServices, AppStore},
        errors::{ApplicationError, ConfigError, WebServerError},
    },
    domains::payments::services::LnEventsService,
    infra::{
        auth::{jwt::JWTAuthenticator, Authenticator},
        database::sea_orm::SeaORMClient,
        lightning::{breez::BreezClient, cln::ClnClient, LightningClient},
    },
};
use humantime::parse_duration;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, warn};

pub struct AppState {
    pub jwt_authenticator: Option<Arc<dyn Authenticator>>,
    pub services: AppServices,
    pub timeout_layer: TimeoutLayer,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, ApplicationError> {
        let timeout_request = parse_duration(&config.web.request_timeout)
            .map_err(|e: humantime::DurationError| WebServerError::ParseConfig(e.to_string()))?;

        // Infra
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

        // Adapters
        let store = AppStore::new_sea_orm(db_conn);
        let lightning_client = get_lightning_client(config.clone(), store.clone()).await?;

        // Services
        let services = AppServices::new(config, store, lightning_client);

        Ok(Self {
            jwt_authenticator,
            services,
            timeout_layer,
        })
    }
}

pub async fn get_lightning_client(
    config: AppConfig,
    store: AppStore,
) -> Result<Arc<dyn LightningClient>, ApplicationError> {
    match config.lightning_provider {
        LightningProvider::Breez => {
            let breez_config = config.breez_config.clone().ok_or_else(|| {
                ConfigError::MissingLightningProviderConfig(config.lightning_provider.to_string())
            })?;

            let lightning_events: LnEventsService = LnEventsService::new(store);
            let client = BreezClient::new(breez_config.clone(), Arc::new(lightning_events)).await?;

            info!(
                working_dir = %breez_config.working_dir,
                "Lightning provider: Breez"
            );

            Ok(Arc::new(client))
        }
        LightningProvider::Cln => {
            let cln_config = config.cln_config.clone().ok_or_else(|| {
                ConfigError::MissingLightningProviderConfig(config.lightning_provider.to_string())
            })?;

            let client = ClnClient::new(cln_config.clone()).await?;

            info!(
                endpoint = %cln_config.endpoint,
                "Lightning provider: Core Lightning"
            );

            Ok(Arc::new(client))
        }
    }
}
