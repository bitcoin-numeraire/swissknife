use std::sync::Arc;

use tower_http::timeout::TimeoutLayer;
use tracing::debug;

use crate::{
    application::{
        dtos::{AppConfig, AuthProvider, LightningProvider},
        errors::{ApplicationError, ConfigError},
    },
    domains::{bitcoin::BitcoinWallet, event::EventService},
    infra::{
        database::sea_orm::SeaORMClient,
        jwt::{local::LocalAuthenticator, oauth2::OAuth2Authenticator, JWTAuthenticator},
        lightning::{
            breez::{BreezClient, BreezListener},
            cln::{ClnGrpcClient, ClnGrpcListener, ClnRestClient, ClnRestListener},
            lnd::{LndRestClient, LndWebsocketListener},
            EventsListener, LnClient,
        },
    },
};

use super::AppStore;

#[derive(Clone)]
pub struct AppAdapters {
    pub store: AppStore,
    pub ln_client: Arc<dyn LnClient>,
    pub ln_listener: Option<Arc<dyn EventsListener>>,
    pub timeout_layer: TimeoutLayer,
    pub bitcoin_wallet: Arc<dyn BitcoinWallet>,
    pub jwt_authenticator: Arc<dyn JWTAuthenticator>,
}

impl AppAdapters {
    pub async fn new(config: AppConfig) -> Result<Self, ApplicationError> {
        let AppConfig { web, database, .. } = config.clone();

        let timeout_layer = TimeoutLayer::new(web.request_timeout);
        let db_conn = SeaORMClient::connect(database).await?;
        let store = AppStore::new_sea_orm(db_conn);
        let jwt_authenticator = get_authenticator(config.clone()).await?;
        let lightning = get_ln_client(config, store.clone()).await?;

        Ok(AppAdapters {
            store,
            ln_client: lightning.ln_client,
            ln_listener: lightning.ln_listener,
            timeout_layer,
            bitcoin_wallet: lightning.bitcoin_wallet,
            jwt_authenticator,
        })
    }
}

struct LightningAdapter {
    ln_client: Arc<dyn LnClient>,
    ln_listener: Option<Arc<dyn EventsListener>>,
    bitcoin_wallet: Arc<dyn BitcoinWallet>,
}

async fn get_authenticator(config: AppConfig) -> Result<Arc<dyn JWTAuthenticator>, ApplicationError> {
    match config.auth_provider {
        AuthProvider::OAuth2 => {
            let oauth2_config = config
                .oauth2
                .clone()
                .ok_or_else(|| ConfigError::MissingAuthProviderConfig(config.auth_provider.to_string()))?;

            debug!(
                config = ?oauth2_config,
                "Auth provider: OAuth2"
            );

            let authenticator = OAuth2Authenticator::new(oauth2_config.clone()).await?;
            Ok(Arc::new(authenticator) as Arc<dyn JWTAuthenticator>)
        }
        AuthProvider::Jwt => {
            let jwt_config = config
                .jwt
                .clone()
                .ok_or_else(|| ConfigError::MissingAuthProviderConfig(config.auth_provider.to_string()))?;

            debug!(
                config = ?jwt_config,
                "Auth provider: Local JWT"
            );

            let authenticator = LocalAuthenticator::new(jwt_config.clone()).await?;
            Ok(Arc::new(authenticator) as Arc<dyn JWTAuthenticator>)
        }
    }
}

async fn get_ln_client(config: AppConfig, store: AppStore) -> Result<LightningAdapter, ApplicationError> {
    match config.ln_provider {
        LightningProvider::Breez => {
            let breez_config = config
                .breez_config
                .clone()
                .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

            debug!(config = ?breez_config,"Lightning provider: Breez");

            let events = EventService::new(store);
            let ln_listener = BreezListener::new(events);
            let ln_client = Arc::new(BreezClient::new(breez_config.clone(), Box::new(ln_listener)).await?);
            let bitcoin_wallet = ln_client.clone();

            Ok(LightningAdapter {
                ln_client,
                ln_listener: None,
                bitcoin_wallet,
            })
        }
        LightningProvider::ClnGrpc => {
            let cln_config = config
                .cln_grpc_config
                .clone()
                .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

            debug!(config = ?cln_config, "Lightning provider: Core Lightning gRPC");

            let ln_client = Arc::new(ClnGrpcClient::new(cln_config.clone()).await?);
            let listener: Arc<dyn EventsListener> = Arc::new(ClnGrpcListener::new(cln_config));
            let bitcoin_wallet = ln_client.clone();

            Ok(LightningAdapter {
                ln_client,
                ln_listener: Some(listener),
                bitcoin_wallet,
            })
        }
        LightningProvider::ClnRest => {
            let cln_config = config
                .cln_rest_config
                .clone()
                .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

            debug!(config = ?cln_config, "Lightning provider: Core Lightning REST");

            let ln_client = Arc::new(ClnRestClient::new(cln_config.clone()).await?);
            let listener: Arc<dyn EventsListener> = Arc::new(ClnRestListener::new(cln_config));
            let bitcoin_wallet = ln_client.clone();

            Ok(LightningAdapter {
                ln_client,
                ln_listener: Some(listener),
                bitcoin_wallet,
            })
        }
        LightningProvider::Lnd => {
            let lnd_config = config
                .lnd_config
                .clone()
                .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

            debug!(config = ?lnd_config, "Lightning provider: LND");

            let ln_client = Arc::new(LndRestClient::new(lnd_config.clone()).await?);
            let listener: Arc<dyn EventsListener> = Arc::new(LndWebsocketListener::new(lnd_config).await?);
            let bitcoin_wallet = ln_client.clone();

            Ok(LightningAdapter {
                ln_client,
                ln_listener: Some(listener),
                bitcoin_wallet,
            })
        }
    }
}
