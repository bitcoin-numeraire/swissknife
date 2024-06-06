use std::sync::Arc;

use crate::{
    application::{
        dtos::{AppConfig, LightningProvider},
        entities::{AppServices, AppStore},
        errors::{ApplicationError, ConfigError, WebServerError},
    },
    domains::lightning::services::LnEventsService,
    infra::{
        auth::{jwt::JWTAuthenticator, Authenticator},
        database::sea_orm::SeaORMClient,
        lightning::{
            breez::BreezClient,
            cln::{ClnGrpcClient, ClnRestClient},
            LnClient,
        },
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
        let ln_client = get_ln_client(config.clone(), store.clone()).await?;

        // Services
        let services = AppServices::new(config, store, ln_client);

        Ok(Self {
            jwt_authenticator,
            services,
            timeout_layer,
        })
    }
}

pub async fn get_ln_client(
    config: AppConfig,
    store: AppStore,
) -> Result<Arc<dyn LnClient>, ApplicationError> {
    let ln_events = Arc::new(LnEventsService::new(store));

    match config.ln_provider {
        LightningProvider::Breez => {
            let breez_config = config.breez_config.clone().ok_or_else(|| {
                ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string())
            })?;

            info!(
                working_dir = %breez_config.working_dir,
                "Lightning provider: Breez"
            );

            let client = BreezClient::new(breez_config.clone(), ln_events).await?;

            Ok(Arc::new(client))
        }
        LightningProvider::Cln => {
            let cln_config = config.cln_config.clone().ok_or_else(|| {
                ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string())
            })?;

            info!(
                endpoint = %cln_config.endpoint,
                "Lightning provider: Core Lightning GRPC"
            );

            let client = ClnGrpcClient::new(cln_config.clone(), ln_events).await?;

            Ok(Arc::new(client))
        }
        LightningProvider::ClnRest => {
            let cln_config = config.cln_rest_config.clone().ok_or_else(|| {
                ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string())
            })?;

            info!(
                endpoint = %cln_config.endpoint,
                "Lightning provider: Core Lightning Rest"
            );

            let client = ClnRestClient::new(cln_config.clone()).await?;

            Ok(Arc::new(client))
        }
    }
}
