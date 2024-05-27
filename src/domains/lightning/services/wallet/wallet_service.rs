use crate::{
    application::errors::ApplicationError,
    domains::lightning::{
        adapters::LightningRepository,
        entities::{LightningInvoiceFilter, LightningPaymentFilter, UserBalance, Wallet},
        services::WalletUseCases,
    },
};
use async_trait::async_trait;
use tracing::{debug, trace};

pub struct WalletService {
    pub store: Box<dyn LightningRepository>,
}

impl WalletService {
    pub fn new(store: Box<dyn LightningRepository>) -> Self {
        WalletService { store }
    }
}

const PAYMENTS_LIMIT: u64 = 15;
const INVOICES_LIMIT: u64 = 15;

#[async_trait]
impl WalletUseCases for WalletService {
    async fn get_balance(&self, user_id: String) -> Result<UserBalance, ApplicationError> {
        trace!(user_id, "Fetching balance");

        let balance = self.store.get_balance(None, &user_id).await?;

        debug!(user_id, "Balance fetched successfully");
        Ok(balance)
    }

    async fn get(&self, user_id: String) -> Result<Wallet, ApplicationError> {
        trace!(user_id, "Fetching wallet");

        let balance = self.store.get_balance(None, &user_id).await?;
        let payments = self
            .store
            .find_payments(LightningPaymentFilter {
                user_id: Some(user_id.clone()),
                limit: Some(PAYMENTS_LIMIT),
                ..Default::default()
            })
            .await?;
        let invoices = self
            .store
            .find_invoices(LightningInvoiceFilter {
                user_id: Some(user_id.clone()),
                limit: Some(INVOICES_LIMIT),
                ..Default::default()
            })
            .await?;
        let address = self.store.find_address_by_user_id(&user_id).await?;

        debug!(user_id, "wallet fetched successfully");
        Ok(Wallet {
            user_balance: balance,
            payments,
            invoices,
            address,
        })
    }
}
