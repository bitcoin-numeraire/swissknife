use std::sync::Arc;

use crate::domains::{
    bitcoin::{BtcAddressRepository, BtcOutputRepository},
    event::EventProjectionUnitOfWork,
    invoice::InvoiceRepository,
    ln_address::LnAddressRepository,
    payment::{PaymentRepository, PaymentUnitOfWork},
    system::{ConfigRepository, HealthProbe},
    user::ApiKeyRepository,
    wallet::WalletRepository,
};

#[derive(Clone)]
pub struct AppStore {
    pub ln_address: Arc<dyn LnAddressRepository>,
    pub payment: Arc<dyn PaymentRepository>,
    pub invoice: Arc<dyn InvoiceRepository>,
    pub wallet: Arc<dyn WalletRepository>,
    pub api_key: Arc<dyn ApiKeyRepository>,
    pub config: Arc<dyn ConfigRepository>,
    pub btc_address: Arc<dyn BtcAddressRepository>,
    pub btc_output: Arc<dyn BtcOutputRepository>,
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
        api_key: Arc<dyn ApiKeyRepository>,
        config: Arc<dyn ConfigRepository>,
        btc_address: Arc<dyn BtcAddressRepository>,
        btc_output: Arc<dyn BtcOutputRepository>,
        health: Arc<dyn HealthProbe>,
        payment_uow: Arc<dyn PaymentUnitOfWork>,
        event_uow: Arc<dyn EventProjectionUnitOfWork>,
    ) -> Self {
        Self {
            ln_address,
            payment,
            invoice,
            wallet,
            api_key,
            config,
            btc_address,
            btc_output,
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
    pub api_key: crate::domains::user::MockApiKeyRepository,
    pub config: crate::domains::system::MockConfigRepository,
    pub btc_address: crate::domains::bitcoin::MockBtcAddressRepository,
    pub btc_output: crate::domains::bitcoin::MockBtcOutputRepository,
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
            api_key: crate::domains::user::MockApiKeyRepository::new(),
            config: crate::domains::system::MockConfigRepository::new(),
            btc_address: crate::domains::bitcoin::MockBtcAddressRepository::new(),
            btc_output: crate::domains::bitcoin::MockBtcOutputRepository::new(),
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
            Arc::new(self.api_key),
            Arc::new(self.config),
            Arc::new(self.btc_address),
            Arc::new(self.btc_output),
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
