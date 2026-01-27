use std::sync::Arc;

use crate::{
    application::{
        dtos::{AppConfig, AuthProvider, LightningProvider},
        entities::{AppServices, AppStore, LnNodeClient},
        errors::{ApplicationError, ConfigError},
    },
    domains::{
        bitcoin::{BitcoinEventsService, BitcoinWallet},
        ln_node::{LnEventsService, LnEventsUseCases},
    },
    infra::{
        database::sea_orm::SeaORMClient,
        jwt::{local::LocalAuthenticator, oauth2::OAuth2Authenticator, JWTAuthenticator},
        lightning::{
            breez::{BreezClient, BreezListener},
            cln::{ClnGrpcClient, ClnGrpcListener, ClnRestClient, ClnRestListener},
            lnd::{LndRestClient, LndWebsocketListener},
            LnClient, LnNodeListener,
        },
    },
};
use tower_http::timeout::TimeoutLayer;
use tracing::{debug, info};

pub struct AppState {
    pub services: AppServices,
    pub ln_client: Arc<dyn LnClient>,
    pub ln_node_client: LnNodeClient,
    #[allow(dead_code)]
    pub ln_listener: Option<Arc<dyn LnNodeListener>>,
    pub timeout_layer: TimeoutLayer,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, ApplicationError> {
        info!("Numeraire SwissKnife version: {}", env!("CARGO_PKG_VERSION"));

        // Infra
        let timeout_layer = TimeoutLayer::new(config.web.request_timeout);
        let db_conn = SeaORMClient::connect(config.database.clone()).await?;
        let jwt_authenticator = get_authenticator(config.clone()).await?;

        // Adapters
        let store = AppStore::new_sea_orm(db_conn);
        let ln_events = Arc::new(LnEventsService::new(store.clone()));
        let bitcoin_events = Arc::new(BitcoinEventsService::new(store.clone()));
        let lightning = get_ln_client(config.clone(), ln_events.clone()).await?;
        let ln_node_client = lightning.ln_node_client;
        let ln_client = match ln_node_client.clone() {
            LnNodeClient::Breez(client) => client as Arc<dyn LnClient>,
            LnNodeClient::ClnGrpc(client) => client as Arc<dyn LnClient>,
            LnNodeClient::ClnRest(client) => client as Arc<dyn LnClient>,
            LnNodeClient::Lnd(client) => client as Arc<dyn LnClient>,
        };
        let bitcoin_wallet = lightning.bitcoin_wallet;

        if let Some(listener) = lightning.ln_listener.clone() {
            let ln_events = ln_events.clone();
            let bitcoin_events = bitcoin_events.clone();
            let bitcoin_wallet = bitcoin_wallet.clone();
            tokio::spawn(async move {
                if let Err(err) = listener.listen(ln_events, bitcoin_events, bitcoin_wallet).await {
                    tracing::error!(%err, "Lightning listener failed");
                }
            });
        }

        // Services
        let services = AppServices::new(
            config,
            store,
            ln_client.clone(),
            bitcoin_wallet,
            bitcoin_events,
            jwt_authenticator,
        );

        Ok(Self {
            services,
            ln_client,
            ln_node_client,
            ln_listener: lightning.ln_listener,
            timeout_layer,
        })
    }
}

struct LightningAdapter {
    ln_node_client: LnNodeClient,
    ln_listener: Option<Arc<dyn LnNodeListener>>,
    bitcoin_wallet: Arc<dyn BitcoinWallet>,
}

async fn get_ln_client(
    config: AppConfig,
    ln_events: Arc<dyn LnEventsUseCases>,
) -> Result<LightningAdapter, ApplicationError> {
    match config.ln_provider {
        LightningProvider::Breez => {
            let breez_config = config
                .breez_config
                .clone()
                .ok_or_else(|| ConfigError::MissingLightningProviderConfig(config.ln_provider.to_string()))?;

            debug!(config = ?breez_config,"Lightning provider: Breez");

            let listener = BreezListener::new(ln_events);
            let client = Arc::new(BreezClient::new(breez_config.clone(), Box::new(listener)).await?);
            let bitcoin_wallet = client.clone() as Arc<dyn BitcoinWallet>;

            Ok(LightningAdapter {
                ln_node_client: LnNodeClient::Breez(client),
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

            let client = Arc::new(ClnGrpcClient::new(cln_config.clone()).await?);
            let listener: Arc<dyn LnNodeListener> = Arc::new(ClnGrpcListener::new(cln_config));
            let bitcoin_wallet = client.clone() as Arc<dyn BitcoinWallet>;

            Ok(LightningAdapter {
                ln_node_client: LnNodeClient::ClnGrpc(client),
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

            let client = Arc::new(ClnRestClient::new(cln_config.clone()).await?);
            let listener: Arc<dyn LnNodeListener> = Arc::new(ClnRestListener::new(cln_config));
            let bitcoin_wallet = client.clone() as Arc<dyn BitcoinWallet>;

            Ok(LightningAdapter {
                ln_node_client: LnNodeClient::ClnRest(client),
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

            let client = Arc::new(LndRestClient::new(lnd_config.clone()).await?);
            let listener: Arc<dyn LnNodeListener> = Arc::new(LndWebsocketListener::new(lnd_config).await?);
            let bitcoin_wallet = client.clone() as Arc<dyn BitcoinWallet>;

            Ok(LightningAdapter {
                ln_node_client: LnNodeClient::Lnd(client),
                ln_listener: Some(listener),
                bitcoin_wallet,
            })
        }
    }
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
            Ok(Arc::new(authenticator))
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
            Ok(Arc::new(authenticator))
        }
    }
}
