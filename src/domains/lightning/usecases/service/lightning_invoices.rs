use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{
            entities::{LightningInvoice, LightningInvoiceStatus},
            usecases::LightningInvoicesUseCases,
        },
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

#[async_trait]
impl LightningInvoicesUseCases for LightningService {
    async fn generate_invoice(
        &self,
        user: AuthUser,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<LightningInvoice, ApplicationError> {
        debug!(user_id = user.sub, "Generating lightning invoice");

        let mut invoice = self
            .lightning_client
            .invoice(amount, description.clone().unwrap_or_default(), expiry)
            .await?;
        invoice.status = LightningInvoiceStatus::PENDING;
        invoice.user_id = user.sub.clone();
        invoice.description = description;

        let invoice = self.store.insert_invoice(invoice).await?;

        info!(
            user_id = user.sub,
            "Lightning invoice generated successfully"
        );

        Ok(invoice)
    }

    async fn get_invoice(
        &self,
        user: AuthUser,
        payment_hash: String,
    ) -> Result<LightningInvoice, ApplicationError> {
        debug!(
            user_id = user.sub,
            payment_hash, "Fetching lightning invoice"
        );

        let lightning_invoice = self
            .store
            .find_invoice(&payment_hash)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        if lightning_invoice.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAccounts)?;
        }

        info!(
            user_id = user.sub,
            payment_hash, "Lightning invoice fetched successfully"
        );
        Ok(lightning_invoice)
    }

    async fn list_invoices(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningInvoice>, ApplicationError> {
        debug!(
            user_id = user.sub,
            limit, offset, "Listing lightning invoices"
        );

        let lightning_invoices = if user.has_permission(Permission::ReadLightningAccounts) {
            // The user has permission to view all addresses
            self.store.find_all_invoices(None, limit, offset).await?
        } else {
            // The user can only view their own payments
            self.store
                .find_all_invoices(Some(user.sub.clone()), limit, offset)
                .await?
        };

        info!(user_id = user.sub, "Lightning invoices listed successfully");
        Ok(lightning_invoices)
    }
}
