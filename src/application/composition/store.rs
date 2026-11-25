use std::sync::Arc;

use crate::domains::{
    account::{AccountRepository, ApiKeyRepository, LocalCredentialRepository},
    asset::AssetRepository,
    bitcoin::{BtcAddressRepository, BtcOutputRepository},
    event::{ClientEventRepository, EventProjectionUnitOfWork},
    invoice::InvoiceRepository,
    ln_address::LnAddressRepository,
    notification::NotificationRepository,
    payment::{PaymentRepository, PaymentUnitOfWork},
    system::{ConfigRepository, HealthProbe},
    wallet::WalletRepository,
};

#[derive(Clone)]
pub struct AppStore {
    pub ln_address: Arc<dyn LnAddressRepository>,
    pub payment: Arc<dyn PaymentRepository>,
    pub invoice: Arc<dyn InvoiceRepository>,
    pub agent_keys: Arc<dyn crate::domains::agent::AgentKeyRepository>,
    pub spending_policies: Arc<dyn crate::domains::agent::SpendingPolicyRepository>,
    pub offer: Arc<dyn crate::domains::offer::OfferRepository>,
    pub wallet: Arc<dyn WalletRepository>,
    pub account: Arc<dyn AccountRepository>,
    pub local_credential: Arc<dyn LocalCredentialRepository>,
    pub asset: Arc<dyn AssetRepository>,
    pub api_key: Arc<dyn ApiKeyRepository>,
    pub config: Arc<dyn ConfigRepository>,
    pub btc_address: Arc<dyn BtcAddressRepository>,
    pub btc_output: Arc<dyn BtcOutputRepository>,
    pub client_event: Arc<dyn ClientEventRepository>,
    pub notification: Arc<dyn NotificationRepository>,
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
        agent_keys: Arc<dyn crate::domains::agent::AgentKeyRepository>,
        spending_policies: Arc<dyn crate::domains::agent::SpendingPolicyRepository>,
        offer: Arc<dyn crate::domains::offer::OfferRepository>,
        wallet: Arc<dyn WalletRepository>,
        account: Arc<dyn AccountRepository>,
        local_credential: Arc<dyn LocalCredentialRepository>,
        asset: Arc<dyn AssetRepository>,
        api_key: Arc<dyn ApiKeyRepository>,
        config: Arc<dyn ConfigRepository>,
        btc_address: Arc<dyn BtcAddressRepository>,
        btc_output: Arc<dyn BtcOutputRepository>,
        client_event: Arc<dyn ClientEventRepository>,
        notification: Arc<dyn NotificationRepository>,
        health: Arc<dyn HealthProbe>,
        payment_uow: Arc<dyn PaymentUnitOfWork>,
        event_uow: Arc<dyn EventProjectionUnitOfWork>,
    ) -> Self {
        Self {
            ln_address,
            payment,
            invoice,
            agent_keys,
            spending_policies,
            offer,
            wallet,
            account,
            local_credential,
            asset,
            api_key,
            config,
            btc_address,
            btc_output,
            client_event,
            notification,
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
    pub agent_keys: crate::domains::agent::MockAgentKeyRepository,
    pub spending_policies: crate::domains::agent::MockSpendingPolicyRepository,
    pub offer: crate::domains::offer::MockOfferRepository,
    pub wallet: crate::domains::wallet::MockWalletRepository,
    pub account: crate::domains::account::MockAccountRepository,
    pub local_credential: crate::domains::account::MockLocalCredentialRepository,
    pub asset: crate::domains::asset::MockAssetRepository,
    pub api_key: crate::domains::account::MockApiKeyRepository,
    pub config: crate::domains::system::MockConfigRepository,
    pub btc_address: crate::domains::bitcoin::MockBtcAddressRepository,
    pub btc_output: crate::domains::bitcoin::MockBtcOutputRepository,
    pub client_event: crate::domains::event::MockClientEventRepository,
    pub notification: crate::domains::notification::MockNotificationRepository,
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
            agent_keys: crate::domains::agent::MockAgentKeyRepository::new(),
            spending_policies: crate::domains::agent::MockSpendingPolicyRepository::new(),
            offer: crate::domains::offer::MockOfferRepository::new(),
            wallet: crate::domains::wallet::MockWalletRepository::new(),
            account: crate::domains::account::MockAccountRepository::new(),
            local_credential: crate::domains::account::MockLocalCredentialRepository::new(),
            asset: crate::domains::asset::MockAssetRepository::new(),
            api_key: crate::domains::account::MockApiKeyRepository::new(),
            config: crate::domains::system::MockConfigRepository::new(),
            btc_address: crate::domains::bitcoin::MockBtcAddressRepository::new(),
            btc_output: crate::domains::bitcoin::MockBtcOutputRepository::new(),
            client_event: crate::domains::event::MockClientEventRepository::new(),
            notification: crate::domains::notification::MockNotificationRepository::new(),
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
            Arc::new(self.agent_keys),
            Arc::new(self.spending_policies),
            Arc::new(self.offer),
            Arc::new(self.wallet),
            Arc::new(self.account),
            Arc::new(self.local_credential),
            Arc::new(self.asset),
            Arc::new(self.api_key),
            Arc::new(self.config),
            Arc::new(self.btc_address),
            Arc::new(self.btc_output),
            Arc::new(self.client_event),
            Arc::new(self.notification),
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
