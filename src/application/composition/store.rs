use std::sync::Arc;

use crate::domains::{
    account::{AccountRepository, ApiKeyRepository},
    asset::AssetRepository,
    bitcoin::{BtcAddressRepository, BtcOutputRepository},
    event::{ClientEventRepository, EventProjectionUnitOfWork, WebhookRepository},
    invoice::InvoiceRepository,
    ln_address::LnAddressRepository,
    payment::{PaymentRepository, PaymentUnitOfWork},
    system::{ConfigRepository, HealthProbe},
    wallet::WalletRepository,
};

#[derive(Clone)]
pub struct AppStore {
    pub ln_address: Arc<dyn LnAddressRepository>,
    pub payment: Arc<dyn PaymentRepository>,
    pub invoice: Arc<dyn InvoiceRepository>,
    pub wallet: Arc<dyn WalletRepository>,
    pub account: Arc<dyn AccountRepository>,
    pub asset: Arc<dyn AssetRepository>,
    pub api_key: Arc<dyn ApiKeyRepository>,
    pub config: Arc<dyn ConfigRepository>,
    pub btc_address: Arc<dyn BtcAddressRepository>,
    pub btc_output: Arc<dyn BtcOutputRepository>,
    pub client_event: Arc<dyn ClientEventRepository>,
    pub webhook: Arc<dyn WebhookRepository>,
    pub health: Arc<dyn HealthProbe>,
    pub payment_uow: Arc<dyn PaymentUnitOfWork>,
    pub event_uow: Arc<dyn EventProjectionUnitOfWork>,
}

impl AppStore {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ln_address: Arc<dyn LnAddressRepository>,
        payment: Arc<dyn PaymentRepository>,
        invoice: Arc<dyn InvoiceRepository>,
        wallet: Arc<dyn WalletRepository>,
        account: Arc<dyn AccountRepository>,
        asset: Arc<dyn AssetRepository>,
        api_key: Arc<dyn ApiKeyRepository>,
        config: Arc<dyn ConfigRepository>,
        btc_address: Arc<dyn BtcAddressRepository>,
        btc_output: Arc<dyn BtcOutputRepository>,
        client_event: Arc<dyn ClientEventRepository>,
        webhook: Arc<dyn WebhookRepository>,
        health: Arc<dyn HealthProbe>,
        payment_uow: Arc<dyn PaymentUnitOfWork>,
        event_uow: Arc<dyn EventProjectionUnitOfWork>,
    ) -> Self {
        Self {
            ln_address,
            payment,
            invoice,
            wallet,
            account,
            asset,
            api_key,
            config,
            btc_address,
            btc_output,
            client_event,
            webhook,
            health,
            payment_uow,
            event_uow,
        }
    }
}

#[cfg(test)]
pub struct MockAppStoreBuilder {
    pub ln_address: crate::domains::ln_address::MockLnAddressRepository,
    pub payment: crate::domains::payment::MockPaymentRepository,
    pub invoice: crate::domains::invoice::MockInvoiceRepository,
    pub wallet: crate::domains::wallet::MockWalletRepository,
    pub account: crate::domains::account::MockAccountRepository,
    pub asset: crate::domains::asset::MockAssetRepository,
    pub api_key: crate::domains::account::MockApiKeyRepository,
    pub config: crate::domains::system::MockConfigRepository,
    pub btc_address: crate::domains::bitcoin::MockBtcAddressRepository,
    pub btc_output: crate::domains::bitcoin::MockBtcOutputRepository,
    pub client_event: crate::domains::event::MockClientEventRepository,
    pub webhook: crate::domains::event::MockWebhookRepository,
    pub health: crate::domains::system::MockHealthProbe,
    pub payment_uow: crate::domains::payment::MockPaymentUnitOfWork,
    pub event_uow: crate::domains::event::MockEventProjectionUnitOfWork,
}

#[cfg(test)]
impl MockAppStoreBuilder {
    pub fn new() -> Self {
        Self {
            ln_address: crate::domains::ln_address::MockLnAddressRepository::new(),
            payment: crate::domains::payment::MockPaymentRepository::new(),
            invoice: crate::domains::invoice::MockInvoiceRepository::new(),
            wallet: crate::domains::wallet::MockWalletRepository::new(),
            account: crate::domains::account::MockAccountRepository::new(),
            asset: crate::domains::asset::MockAssetRepository::new(),
            api_key: crate::domains::account::MockApiKeyRepository::new(),
            config: crate::domains::system::MockConfigRepository::new(),
            btc_address: crate::domains::bitcoin::MockBtcAddressRepository::new(),
            btc_output: crate::domains::bitcoin::MockBtcOutputRepository::new(),
            client_event: crate::domains::event::MockClientEventRepository::new(),
            webhook: crate::domains::event::MockWebhookRepository::new(),
            health: crate::domains::system::MockHealthProbe::new(),
            payment_uow: crate::domains::payment::MockPaymentUnitOfWork::new(),
            event_uow: crate::domains::event::MockEventProjectionUnitOfWork::new(),
        }
    }

    pub fn build(self) -> AppStore {
        AppStore::new(
            Arc::new(self.ln_address),
            Arc::new(self.payment),
            Arc::new(self.invoice),
            Arc::new(self.wallet),
            Arc::new(self.account),
            Arc::new(self.asset),
            Arc::new(self.api_key),
            Arc::new(self.config),
            Arc::new(self.btc_address),
            Arc::new(self.btc_output),
            Arc::new(self.client_event),
            Arc::new(self.webhook),
            Arc::new(self.health),
            Arc::new(self.payment_uow),
            Arc::new(self.event_uow),
        )
    }
}

#[cfg(test)]
impl Default for MockAppStoreBuilder {
    fn default() -> Self {
        Self::new()
    }
}
