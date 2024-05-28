use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::lightning::{
        entities::{Invoice, InvoiceFilter},
        services::InvoicesUseCases,
    },
};

use super::LightningService;

const DEFAULT_INVOICE_DESCRIPTION: &str = "Numeraire Swissknife Invoice";

#[async_trait]
impl InvoicesUseCases for LightningService {
    async fn generate_invoice(
        &self,
        user_id: String,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<Invoice, ApplicationError> {
        debug!(%user_id, "Generating lightning invoice");

        let mut invoice = self
            .lightning_client
            .invoice(
                amount,
                description.unwrap_or(DEFAULT_INVOICE_DESCRIPTION.to_string()),
                expiry.unwrap_or(self.invoice_expiry),
            )
            .await?;
        invoice.user_id = user_id.clone();

        let invoice = self.store.insert_invoice(None, invoice).await?;

        info!(
            id = invoice.id.to_string(),
            "Lightning invoice generated successfully"
        );

        Ok(invoice)
    }

    async fn get_invoice(&self, id: Uuid) -> Result<Invoice, ApplicationError> {
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

    async fn list_invoices(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, ApplicationError> {
        trace!(?filter, "Listing lightning invoices");

        let lightning_invoices = self.store.find_invoices(filter.clone()).await?;

        debug!(?filter, "Lightning invoices listed successfully");
        Ok(lightning_invoices)
    }

    async fn delete_invoice(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting lightning invoice");

        let n_deleted = self
            .store
            .delete_invoices(InvoiceFilter {
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

    async fn delete_invoices(&self, filter: InvoiceFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting lightning invoices");

        let n_deleted = self.store.delete_invoices(filter.clone()).await?;

        info!(
            ?filter,
            n_deleted, "Lightning invoices deleted successfully"
        );
        Ok(n_deleted)
    }
}
