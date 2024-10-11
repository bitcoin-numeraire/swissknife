use std::sync::Arc;

use crate::{
    application::{dtos::AppConfig, errors::ConfigError},
    domains::{
        invoice::{InvoiceService, InvoiceUseCases},
        ln_address::{LnAddressService, LnAddressUseCases},
        lnurl::{LnUrlService, LnUrlUseCases},
        nostr::{NostrService, NostrUseCases},
        payment::{PaymentService, PaymentsUseCases},
        system::{SystemService, SystemUseCases},
        user::{ApiKeyService, ApiKeyUseCases, AuthService, AuthUseCases},
        wallet::{WalletService, WalletUseCases},
    },
    infra::{
        jwt::JWTAuthenticator,
        lightning::{
            breez::BreezClient,
            cln::{ClnGrpcClient, ClnRestClient},
            LnClient,
        },
    },
};

use super::AppStore;

pub struct AppServices {
    pub invoice: Box<dyn InvoiceUseCases>,
    pub payment: Box<dyn PaymentsUseCases>,
    pub wallet: Box<dyn WalletUseCases>,
    pub lnurl: Box<dyn LnUrlUseCases>,
    pub ln_address: Box<dyn LnAddressUseCases>,
    pub auth: Box<dyn AuthUseCases>,
    pub system: Box<dyn SystemUseCases>,
    pub nostr: Box<dyn NostrUseCases>,
    pub api_key: Box<dyn ApiKeyUseCases>,
}

impl AppServices {
    pub fn new(
        config: AppConfig,
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        jwt_authenticator: Arc<dyn JWTAuthenticator>,
    ) -> Self {
        let payments = PaymentService::new(
            store.clone(),
            ln_client.clone(),
            config.domain.clone(),
            config.fee_buffer.unwrap_or_default(),
        );
        let invoices = InvoiceService::new(
            store.clone(),
            ln_client.clone(),
            config.invoice_expiry.as_secs() as u32,
            config.ln_provider,
        );
        let lnurl = LnUrlService::new(
            store.clone(),
            ln_client.clone(),
            config.invoice_expiry.as_secs() as u32,
            config.domain,
            config.host,
        );
        let ln_address = LnAddressService::new(store.clone());
        let wallet = WalletService::new(store.clone());
        let auth = AuthService::new(jwt_authenticator, store.clone());
        let system = SystemService::new(store.clone(), ln_client);
        let nostr = NostrService::new(store.clone());
        let api_key = ApiKeyService::new(store);

        AppServices {
            invoice: Box::new(invoices),
            payment: Box::new(payments),
            wallet: Box::new(wallet),
            lnurl: Box::new(lnurl),
            ln_address: Box::new(ln_address),
            auth: Box::new(auth),
            system: Box::new(system),
            nostr: Box::new(nostr),
            api_key: Box::new(api_key),
        }
    }
}

#[derive(Clone)]
pub enum LnNodeClient {
    Breez(Arc<BreezClient>),
    ClnGrpc(Arc<ClnGrpcClient>),
    ClnRest(Arc<ClnRestClient>),
}

impl LnNodeClient {
    pub fn as_breez_client(&self) -> Result<&BreezClient, ConfigError> {
        if let LnNodeClient::Breez(client) = self {
            Ok(client)
        } else {
            Err(ConfigError::InvalidLightningProvider("Breez".to_string()))
        }
    }
}
