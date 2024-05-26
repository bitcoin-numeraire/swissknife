use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        dtos::LightningInvoiceFilter,
        errors::{ApplicationError, DataError},
    },
    domains::lightning::{entities::LightningInvoice, services::LightningInvoicesUseCases},
};

use super::LightningService;

#[async_trait]
impl LightningInvoicesUseCases for LightningService {
    async fn generate_invoice(
        &self,
        user_id: String,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<LightningInvoice, ApplicationError> {
        debug!(%user_id, "Generating lightning invoice");

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
        invoice.user_id = user_id.clone();

        let invoice = self.store.insert_invoice(invoice).await?;

        info!(
            id = invoice.id.to_string(),
            "Lightning invoice generated successfully"
        );

        Ok(invoice)
    }

    async fn get_invoice(&self, id: Uuid) -> Result<LightningInvoice, ApplicationError> {
        trace!(
            %id,
            "Fetching lightning invoice"
        );

        let lightning_invoice = self
            .store
            .find_invoice(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        debug!(
            %id, "Lightning invoice fetched successfully"
        );
        Ok(lightning_invoice)
    }

    async fn list_invoices(
        &self,
        filter: LightningInvoiceFilter,
    ) -> Result<Vec<LightningInvoice>, ApplicationError> {
        trace!(?filter, "Listing lightning invoices");

        let lightning_invoices = self.store.find_invoices(filter.clone()).await?;

        debug!(?filter, "Lightning invoices listed successfully");
        Ok(lightning_invoices)
    }

    async fn delete_invoice(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting lightning invoice");

        let n_deleted = self
            .store
            .delete_invoices(LightningInvoiceFilter {
                id: Some(id),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Lightning invoice not found.".to_string()).into());
        }

        info!(%id, "Lightning invoice deleted successfully");
        Ok(())
    }

    async fn delete_invoices(
        &self,
        filter: LightningInvoiceFilter,
    ) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting lightning invoices");

        let n_deleted = self.store.delete_invoices(filter.clone()).await?;

        info!(
            ?filter,
            n_deleted, "Lightning invoices deleted successfully"
        );
        Ok(n_deleted)
    }
}
