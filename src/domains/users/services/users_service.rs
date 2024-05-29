use crate::{
    application::{
        entities::{AppStore, PaginationFilter},
        errors::ApplicationError,
    },
    domains::{
        invoices::entities::InvoiceFilter,
        payments::entities::PaymentFilter,
        users::{
            entities::{UserBalance, Wallet},
            services::WalletUseCases,
        },
    },
};
use async_trait::async_trait;
use tracing::{debug, trace};

pub struct WalletService {
    store: AppStore,
}

impl WalletService {
    pub fn new(store: AppStore) -> Self {
        WalletService { store }
    }
}

const PAYMENTS_LIMIT: u64 = 10;
const INVOICES_LIMIT: u64 = 10;

#[async_trait]
impl WalletUseCases for WalletService {
    async fn get_balance(&self, user_id: String) -> Result<UserBalance, ApplicationError> {
        trace!(user_id, "Fetching balance");

        let balance = self.store.user.get_balance(None, &user_id).await?;

        debug!(user_id, "Balance fetched successfully");
        Ok(balance)
    }

    async fn get(&self, user_id: String) -> Result<Wallet, ApplicationError> {
        trace!(user_id, "Fetching wallet");

        let balance = self.store.user.get_balance(None, &user_id).await?;
        let payments = self
            .store
            .payment
            .find_many(PaymentFilter {
                user_id: Some(user_id.clone()),
                pagination: PaginationFilter {
                    limit: Some(PAYMENTS_LIMIT),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;
        let invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                user_id: Some(user_id.clone()),
                pagination: PaginationFilter {
                    limit: Some(INVOICES_LIMIT),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;
        let address = self
            .store
            .lightning
            .find_address_by_user_id(&user_id)
            .await?;

        debug!(user_id, "wallet fetched successfully");
        Ok(Wallet {
            user_balance: balance,
            payments,
            invoices,
            address,
        })
    }
}
