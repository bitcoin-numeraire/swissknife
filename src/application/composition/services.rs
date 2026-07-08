use std::sync::Arc;

use crate::{
    application::composition::{AppAdapters, AppConfig},
    domains::{
        bitcoin::{BitcoinService, BitcoinUseCases},
        event::{EventService, EventUseCases},
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
    pub system: Arc<dyn SystemUseCases>,
    pub nostr: Box<dyn NostrUseCases>,
    pub api_key: Box<dyn ApiKeyUseCases>,
    pub bitcoin: Box<dyn BitcoinUseCases>,
    pub event: Arc<dyn EventUseCases>,
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

        let event = Arc::new(EventService::new(store.clone()));
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
        let active_bitcoin_network = bitcoin_wallet.network();
        let wallet = WalletService::new(store.clone());
        let auth = AuthService::new(jwt_authenticator, store.clone(), auth_provider, active_bitcoin_network);
        let system = Arc::new(SystemService::new(store.clone(), ln_client.clone()));
        let nostr = NostrService::new(store.clone());
        let api_key = ApiKeyService::new(store.clone());
        let bitcoin = BitcoinService::new(
            store.clone(),
            bitcoin_wallet,
            bitcoin_address_type,
            event.clone(),
            system.clone(),
        );

        AppServices {
            invoice: Box::new(invoices),
            payment: Box::new(payments),
            wallet: Box::new(wallet),
            lnurl: Box::new(lnurl),
            ln_address: Box::new(ln_address),
            auth: Box::new(auth),
            system,
            nostr: Box::new(nostr),
            api_key: Box::new(api_key),
            bitcoin: Box::new(bitcoin),
            event,
        }
    }
}

/// Test-only builder that assembles an [`AppServices`] from generated `mockall`
/// use-case mocks, mirroring `MockAppStoreBuilder`. Configure expectations on the
/// public mock fields, then call [`MockAppServicesBuilder::build`] to obtain an
/// `AppServices` suitable for handler unit tests.
#[cfg(test)]
pub struct MockAppServicesBuilder {
    pub invoice: crate::domains::invoice::MockInvoiceUseCases,
    pub payment: crate::domains::payment::MockPaymentsUseCases,
    pub wallet: crate::domains::wallet::MockWalletUseCases,
    pub lnurl: crate::domains::lnurl::MockLnUrlUseCases,
    pub ln_address: crate::domains::ln_address::MockLnAddressUseCases,
    pub auth: crate::domains::user::MockAuthUseCases,
    pub system: crate::domains::system::MockSystemUseCases,
    pub nostr: crate::domains::nostr::MockNostrUseCases,
    pub api_key: crate::domains::user::MockApiKeyUseCases,
    pub bitcoin: crate::domains::bitcoin::MockBitcoinUseCases,
    pub event: crate::domains::event::MockEventUseCases,
}

#[cfg(test)]
impl MockAppServicesBuilder {
    pub fn new() -> Self {
        Self {
            invoice: crate::domains::invoice::MockInvoiceUseCases::new(),
            payment: crate::domains::payment::MockPaymentsUseCases::new(),
            wallet: crate::domains::wallet::MockWalletUseCases::new(),
            lnurl: crate::domains::lnurl::MockLnUrlUseCases::new(),
            ln_address: crate::domains::ln_address::MockLnAddressUseCases::new(),
            auth: crate::domains::user::MockAuthUseCases::new(),
            system: crate::domains::system::MockSystemUseCases::new(),
            nostr: crate::domains::nostr::MockNostrUseCases::new(),
            api_key: crate::domains::user::MockApiKeyUseCases::new(),
            bitcoin: crate::domains::bitcoin::MockBitcoinUseCases::new(),
            event: crate::domains::event::MockEventUseCases::new(),
        }
    }

    pub fn build(self) -> AppServices {
        AppServices {
            invoice: Box::new(self.invoice),
            payment: Box::new(self.payment),
            wallet: Box::new(self.wallet),
            lnurl: Box::new(self.lnurl),
            ln_address: Box::new(self.ln_address),
            auth: Box::new(self.auth),
            system: Arc::new(self.system),
            nostr: Box::new(self.nostr),
            api_key: Box::new(self.api_key),
            bitcoin: Box::new(self.bitcoin),
            event: Arc::new(self.event),
        }
    }
}

#[cfg(test)]
impl Default for MockAppServicesBuilder {
    fn default() -> Self {
        Self::new()
    }
}
