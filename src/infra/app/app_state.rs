use std::sync::Arc;

use crate::{
    application::{
        dtos::{AppConfig, AuthProvider, LightningProvider},
        entities::{AppServices, AppStore, LnNodeClient},
        errors::{ApplicationError, ConfigError},
    },
    domains::ln_node::{LnEventsService, LnEventsUseCases},
    infra::{
        database::sea_orm::SeaORMClient,
        jwt::{local::LocalAuthenticator, oauth2::OAuth2Authenticator, JWTAuthenticator},
        lightning::{
            breez::BreezClient,
            cln::{ClnGrpcClient, ClnRestClient},
            lnd::LndRestClient,
            LnClient,
        },
    },
};
use tower_http::timeout::TimeoutLayer;
use tracing::info;

pub struct AppState {
    pub services: AppServices,
    pub ln_client: Arc<dyn LnClient>,
    pub ln_node_client: LnNodeClient,
    pub timeout_layer: TimeoutLayer,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, ApplicationError> {
        // Infra
        let timeout_layer = TimeoutLayer::new(config.web.request_timeout);
        let db_conn = SeaORMClient::connect(config.database.clone()).await?;
        let jwt_authenticator = get_authenticator(config.clone()).await?;

        // Adapters
        let store = AppStore::new_sea_orm(db_conn);
        let ln_events = LnEventsService::new(store.clone());
        let ln_node_client = get_ln_client(config.clone(), Arc::new(ln_events)).await?;
        let ln_client = match ln_node_client.clone() {
            LnNodeClient::Breez(client) => client as Arc<dyn LnClient>,
            LnNodeClient::ClnGrpc(client) => client as Arc<dyn LnClient>,
            LnNodeClient::ClnRest(client) => client as Arc<dyn LnClient>,
            LnNodeClient::Lnd(client) => client as Arc<dyn LnClient>,
        };

        // Services
        let services = AppServices::new(config, store, ln_client.clone(), jwt_authenticator);

        Ok(Self {
            services,
            ln_client,
            ln_node_client,
            timeout_layer,
        })
    }
}

async fn get_ln_client(
    config: AppConfig,
    ln_events: Arc<dyn LnEventsUseCases>,
) -> Result<LnNodeClient, ApplicationError> {
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

            Ok(LnNodeClient::Breez(Arc::new(client)))
        }
        LightningProvider::ClnGrpc => {
            let cln_config = config.cln_grpc_config.clone().ok_or_else(|| {
                ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string())
            })?;

            info!(config = ?cln_config, "Lightning provider: Core Lightning gRPC");

            let client = ClnGrpcClient::new(cln_config.clone(), ln_events).await?;

            Ok(LnNodeClient::ClnGrpc(Arc::new(client)))
        }
        LightningProvider::ClnRest => {
            let cln_config = config.cln_rest_config.clone().ok_or_else(|| {
                ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string())
            })?;

            info!(endpoint = %cln_config.endpoint, "Lightning provider: Core Lightning REST");

            let client = ClnRestClient::new(cln_config.clone(), ln_events).await?;

            Ok(LnNodeClient::ClnRest(Arc::new(client)))
        }
        LightningProvider::Lnd => {
            let lnd_config = config.lnd_config.clone().ok_or_else(|| {
                ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string())
            })?;

            info!(config = ?lnd_config, "Lightning provider: LND gRPC");

            let client = LndRestClient::new(lnd_config.clone(), ln_events).await?;

            Ok(LnNodeClient::Lnd(Arc::new(client)))
        }
    }
}

async fn get_authenticator(
    config: AppConfig,
) -> Result<Arc<dyn JWTAuthenticator>, ApplicationError> {
    match config.auth_provider {
        AuthProvider::OAuth2 => {
            let oauth2_config = config.oauth2.clone().ok_or_else(|| {
                ConfigError::MissingAuthProviderConfig(config.auth_provider.to_string())
            })?;

            info!(
                config = ?oauth2_config,
                "Auth provider: OAuth2"
            );

            let authenticator = OAuth2Authenticator::new(oauth2_config.clone()).await?;
            Ok(Arc::new(authenticator))
        }
        AuthProvider::Jwt => {
            let jwt_config = config.jwt.clone().ok_or_else(|| {
                ConfigError::MissingAuthProviderConfig(config.auth_provider.to_string())
            })?;

            info!(
                config = ?jwt_config,
                "Auth provider: Local JWT"
            );

            let authenticator = LocalAuthenticator::new(jwt_config.clone()).await?;
            Ok(Arc::new(authenticator))
        }
    }
}
