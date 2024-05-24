use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{
            entities::{LightningInvoice, LightningInvoiceDeleteFilter},
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

    async fn get_invoice(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningInvoice, ApplicationError> {
        trace!(
            user_id = user.sub,
            %id,
            "Fetching lightning invoice"
        );

        let lightning_invoice = self
            .store
            .find_invoice(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        if lightning_invoice.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAccounts)?;
        }

        debug!(
            user_id = user.sub,
            %id, "Lightning invoice fetched successfully"
        );
        Ok(lightning_invoice)
    }

    async fn list_invoices(
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

        let lightning_invoices = if user.has_permission(Permission::ReadLightningAccounts) {
            // The user has permission to view all addresses
            self.store.find_invoices(None, limit, offset).await?
        } else {
            // The user can only view their own payments
            self.store
                .find_invoices(Some(user.sub.clone()), limit, offset)
                .await?
        };

        debug!(user_id = user.sub, "Lightning invoices listed successfully");
        Ok(lightning_invoices)
    }

    async fn delete_expired_invoices(&self, user: AuthUser) -> Result<u64, ApplicationError> {
        trace!(user_id = user.sub, "Deleting expired lightning invoices");

        let n_deleted = self
            .store
            .delete_invoices(
                Some(user.sub.clone()),
                LightningInvoiceDeleteFilter {
                    expired: Some(true),
                },
            )
            .await?;

        info!(
            user_id = user.sub,
            n_deleted, "Expired lightning invoices deleted successfully"
        );

        Ok(n_deleted)
    }
}
