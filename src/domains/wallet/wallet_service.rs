use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::{invoice::InvoiceFilter, payment::PaymentFilter},
};
use async_trait::async_trait;
use tracing::{debug, trace};
use uuid::Uuid;

use super::{Contact, UserBalance, Wallet, WalletUseCases};

pub struct WalletService {
    store: AppStore,
}

impl WalletService {
    pub fn new(store: AppStore) -> Self {
        WalletService { store }
    }
}

#[async_trait]
impl WalletUseCases for WalletService {
    async fn get_balance(&self, id: Uuid) -> Result<UserBalance, ApplicationError> {
        trace!(%id, "Fetching balance");

        let balance = self.store.wallet.get_balance(None, id).await?;

        debug!(%id, "Balance fetched successfully");
        Ok(balance)
    }

    async fn get(&self, id: Uuid) -> Result<Wallet, ApplicationError> {
        trace!(%id, "Fetching wallet");

        let mut wallet = self
            .store
            .wallet
            .find_by_user_id(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Wallet not found.".to_string()))?;

        wallet.user_balance = self.store.wallet.get_balance(None, id).await?;

        wallet.payments = self
            .store
            .payment
            .find_many(PaymentFilter {
                wallet_id: Some(id.clone()),
                ..Default::default()
            })
            .await?;

        wallet.invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                wallet_id: Some(id.clone()),
                ..Default::default()
            })
            .await?;

        wallet.contacts = self.store.payment.find_contacts(id).await?;

        debug!(%id, "wallet fetched successfully");
        Ok(wallet)
    }

    async fn list_contacts(&self, id: Uuid) -> Result<Vec<Contact>, ApplicationError> {
        trace!(%id, "Fetching contacts");

        let contacts = self.store.payment.find_contacts(id).await?;

        debug!(%id, "Contacts fetched successfully");
        Ok(contacts)
    }
}
