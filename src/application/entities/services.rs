use crate::{
    application::{dtos::AppConfig, entities::AppAdapters},
    domains::{
        bitcoin::{BitcoinService, BitcoinUseCases},
        event::EventService,
        invoice::{InvoiceService, InvoiceUseCases},
        ln_address::{LnAddressService, LnAddressUseCases},
        lnurl::{LnUrlService, LnUrlUseCases},
        nostr::{NostrService, NostrUseCases},
        payment::{PaymentService, PaymentsUseCases},
        system::{SystemService, SystemUseCases},
        user::{ApiKeyService, ApiKeyUseCases, AuthService, AuthUseCases},
        wallet::{WalletService, WalletUseCases},
    },
};

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
    pub bitcoin: Box<dyn BitcoinUseCases>,
    pub event: EventService,
}

impl AppServices {
    pub fn new(config: AppConfig, adapters: AppAdapters) -> Self {
        let AppConfig {
            domain,
            host,
            invoice_expiry,
            fee_buffer,
            auth_provider,
            bitcoin_address_type,
            ..
        } = config;

        let AppAdapters {
            store,
            ln_client,
            bitcoin_wallet,
            jwt_authenticator,
            ..
        } = adapters;

        let event = EventService::new(store.clone());
        let payments = PaymentService::new(
            store.clone(),
            ln_client.clone(),
            bitcoin_wallet.clone(),
            domain.clone(),
            fee_buffer.unwrap_or_default(),
            event.clone(),
        );
        let invoices = InvoiceService::new(
            store.clone(),
            ln_client.clone(),
            bitcoin_wallet.clone(),
            invoice_expiry.as_secs() as u32,
            event.clone(),
        );
        let lnurl = LnUrlService::new(
            store.clone(),
            ln_client.clone(),
            invoice_expiry.as_secs() as u32,
            domain,
            host,
        );
        let ln_address = LnAddressService::new(store.clone());
        let wallet = WalletService::new(store.clone());
        let auth = AuthService::new(jwt_authenticator, store.clone(), auth_provider);
        let system = SystemService::new(store.clone(), ln_client.clone());
        let nostr = NostrService::new(store.clone());
        let api_key = ApiKeyService::new(store.clone());
        let bitcoin = BitcoinService::new(store.clone(), bitcoin_wallet, bitcoin_address_type);
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
            bitcoin: Box::new(bitcoin),
            event,
        }
    }
}
