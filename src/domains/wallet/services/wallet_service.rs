use crate::{
    application::{entities::AppStore, errors::ApplicationError},
    domains::{
        invoices::entities::InvoiceFilter,
        payments::entities::PaymentFilter,
        wallet::entities::{Contact, UserBalance, Wallet},
    },
};
use async_trait::async_trait;
use tracing::{debug, trace};

use super::WalletUseCases;

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
    async fn get_balance(&self, user_id: String) -> Result<UserBalance, ApplicationError> {
        trace!(user_id, "Fetching balance");

        let balance = self.store.wallet.get_balance(None, &user_id).await?;

        debug!(user_id, "Balance fetched successfully");
        Ok(balance)
    }

    async fn get(&self, user_id: String) -> Result<Wallet, ApplicationError> {
        trace!(user_id, "Fetching wallet");

        let balance = self.store.wallet.get_balance(None, &user_id).await?;

        let payments = self
            .store
            .payment
            .find_many(PaymentFilter {
                user_id: Some(user_id.clone()),
                ..Default::default()
            })
            .await?;

        let invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                user_id: Some(user_id.clone()),
                ..Default::default()
            })
            .await?;

        let ln_address = self.store.ln_address.find_by_user_id(&user_id).await?;

        let contacts = self.store.payment.find_contacts(&user_id).await?;

        debug!(user_id, "wallet fetched successfully");
        Ok(Wallet {
            user_balance: balance,
            payments,
            invoices,
            ln_address,
            contacts,
        })
    }

    async fn list_contacts(&self, user_id: String) -> Result<Vec<Contact>, ApplicationError> {
        trace!("Fetching contacts");

        let contacts = self.store.payment.find_contacts(&user_id).await?;

        debug!("Contacts fetched successfully");
        Ok(contacts)
    }
}
