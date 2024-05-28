use crate::{
    application::{entities::PaginationFilter, errors::ApplicationError},
    domains::{
        lightning::{
            adapters::LightningRepository,
            entities::{InvoiceFilter, UserBalance, Wallet},
            services::WalletUseCases,
        },
        payments::{adapters::PaymentRepository, entities::PaymentFilter},
    },
};
use async_trait::async_trait;
use tracing::{debug, trace};

pub struct WalletService {
    pub lightning_repo: Box<dyn LightningRepository>,
    pub payment_repo: Box<dyn PaymentRepository>,
}

impl WalletService {
    pub fn new(
        lightning_repo: Box<dyn LightningRepository>,
        payment_repo: Box<dyn PaymentRepository>,
    ) -> Self {
        WalletService {
            lightning_repo,
            payment_repo,
        }
    }
}

const PAYMENTS_LIMIT: u64 = 15;
const INVOICES_LIMIT: u64 = 15;

#[async_trait]
impl WalletUseCases for WalletService {
    async fn get_balance(&self, user_id: String) -> Result<UserBalance, ApplicationError> {
        trace!(user_id, "Fetching balance");

        let balance = self.lightning_repo.get_balance(None, &user_id).await?;

        debug!(user_id, "Balance fetched successfully");
        Ok(balance)
    }

    async fn get(&self, user_id: String) -> Result<Wallet, ApplicationError> {
        trace!(user_id, "Fetching wallet");

        let balance = self.lightning_repo.get_balance(None, &user_id).await?;
        let payments = self
            .payment_repo
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
            .lightning_repo
            .find_invoices(InvoiceFilter {
                user_id: Some(user_id.clone()),
                pagination: PaginationFilter {
                    limit: Some(INVOICES_LIMIT),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;
        let address = self
            .lightning_repo
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
