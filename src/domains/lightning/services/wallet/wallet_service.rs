use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{
            adapters::LightningRepository,
            entities::{
                LightningInvoice, LightningInvoiceFilter, LightningInvoiceStatus, UserBalance,
            },
            services::WalletUseCases,
        },
        users::entities::AuthUser,
    },
};
use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

pub struct WalletService {
    pub store: Box<dyn LightningRepository>,
}

impl WalletService {
    pub fn new(store: Box<dyn LightningRepository>) -> Self {
        WalletService { store }
    }
}

#[async_trait]
impl WalletUseCases for WalletService {
    async fn get_balance(&self, user: AuthUser) -> Result<UserBalance, ApplicationError> {
        trace!(user_id = user.sub, "Fetching balance");

        let balance = self.store.get_balance(None, &user.sub).await?;

        debug!(user_id = user.sub, "Balance fetched successfully");
        Ok(balance)
    }

    async fn generate_Lightning_invoice(
        &self,
        user: AuthUser,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<LightningInvoice, ApplicationError> {
        debug!(user_id = user.sub, "Generating lightning invoice");

        let description = match description {
            Some(desc) if desc.is_empty() => self.invoice_description.clone(),
            Some(desc) => desc,
            None => self.invoice_description.clone(),
        };

        let mut invoice = self
            .lightning_client
            .invoice(
                amount,
                description.clone(),
                expiry.unwrap_or(self.invoice_expiry),
            )
            .await?;
        invoice.user_id = user.sub.clone();

        let invoice = self.store.insert_invoice(invoice).await?;

        info!(
            user_id = user.sub,
            "Lightning invoice generated successfully"
        );

        Ok(invoice)
    }

    async fn get_lightning_invoice(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningInvoice, ApplicationError> {
        trace!(
            user_id = user.sub,
            %id,
            "Fetching lightning invoice"
        );

        let lightning_invoices = self
            .store
            .find_invoices(LightningInvoiceFilter {
                user_id: Some(user.sub.clone()),
                id: Some(id),
                ..Default::default()
            })
            .await?;

        let lightning_invoice = lightning_invoices
            .first()
            .cloned()
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        debug!(
            user_id = user.sub,
            %id, "Lightning invoice fetched successfully"
        );
        Ok(lightning_invoice)
    }

    async fn list_lightning_invoices(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningInvoice>, ApplicationError> {
        trace!(
            user_id = user.sub,
            limit,
            offset,
            "Listing lightning invoices"
        );

        let lightning_invoices = self
            .store
            .find_invoices(LightningInvoiceFilter {
                user_id: Some(user.sub.clone()),
                limit,
                offset,
                ..Default::default()
            })
            .await?;

        debug!(user_id = user.sub, "Lightning invoices listed successfully");
        Ok(lightning_invoices)
    }

    async fn delete_expired_invoices(&self, user: AuthUser) -> Result<u64, ApplicationError> {
        trace!(user_id = user.sub, "Deleting expired lightning invoices");

        let n_deleted = self
            .store
            .delete_invoices(LightningInvoiceFilter {
                user_id: Some(user.sub.clone()),
                status: Some(LightningInvoiceStatus::EXPIRED),
                ..Default::default()
            })
            .await?;

        info!(
            user_id = user.sub,
            n_deleted, "Expired lightning invoices deleted successfully"
        );

        Ok(n_deleted)
    }
}
