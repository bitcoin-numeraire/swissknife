use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::invoices::entities::{Invoice, InvoiceFilter, LnURLpInvoice, SuccessAction},
};

use std::sync::Arc;

use crate::infra::lightning::LnClient;

use super::InvoiceUseCases;

const DEFAULT_INVOICE_EXPIRY: u32 = 3600;
const DEFAULT_INVOICE_DESCRIPTION: &str = "Numeraire Swissknife Invoice";

pub struct InvoiceService {
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
    invoice_expiry: u32,
    domain: String,
}

impl InvoiceService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        invoice_expiry: Option<u32>,
        domain: String,
    ) -> Self {
        InvoiceService {
            store,
            ln_client,
            invoice_expiry: invoice_expiry.unwrap_or(DEFAULT_INVOICE_EXPIRY),
            domain,
        }
    }
}

#[async_trait]
impl InvoiceUseCases for InvoiceService {
    async fn invoice(
        &self,
        user_id: String,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<Invoice, ApplicationError> {
        debug!(%user_id, "Generating lightning invoice");

        let mut invoice = self
            .ln_client
            .invoice(
                amount,
                description.unwrap_or(DEFAULT_INVOICE_DESCRIPTION.to_string()),
                expiry.unwrap_or(self.invoice_expiry),
            )
            .await?;
        invoice.user_id = user_id.clone();

        let invoice = self.store.invoice.insert(None, invoice).await?;

        info!(
            id = invoice.id.to_string(),
            "Lightning invoice generated successfully"
        );

        Ok(invoice)
    }

    async fn invoice_lnurlp(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<LnURLpInvoice, ApplicationError> {
        debug!(username, amount, comment, "Generating LNURLp invoice");

        let ln_address = self
            .store
            .ln_address
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let mut invoice = self
            .ln_client
            .invoice(
                amount,
                comment.unwrap_or(format!("Payment to {}@{}", username, self.domain)),
                self.invoice_expiry,
            )
            .await?;
        invoice.user_id = ln_address.user_id.clone();
        invoice.ln_address = Some(ln_address.id);

        // TODO: Get or add more information to make this a LNURLp invoice (like fetching a success action specific to the user)
        let invoice = self.store.invoice.insert(None, invoice).await?;
        let lnurlp_invoice = LnURLpInvoice {
            pr: invoice.lightning.unwrap().bolt11,
            success_action: Some(SuccessAction {
                tag: "message".to_string(),
                message: Some("Thanks for the sats!".to_string()),
            }),
            disposable: None,
            routes: vec![],
        };

        info!(username, "Lightning invoice generated successfully");
        Ok(lnurlp_invoice)
    }

    async fn get(&self, id: Uuid) -> Result<Invoice, ApplicationError> {
        trace!(
            %id,
            "Fetching lightning invoice"
        );

        let lightning_invoice = self
            .store
            .invoice
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        debug!(
            %id, "Lightning invoice fetched successfully"
        );
        Ok(lightning_invoice)
    }

    async fn list(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, ApplicationError> {
        trace!(?filter, "Listing lightning invoices");

        let lightning_invoices = self.store.invoice.find_many(filter.clone()).await?;

        debug!(?filter, "Lightning invoices listed successfully");
        Ok(lightning_invoices)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting lightning invoice");

        let n_deleted = self
            .store
            .invoice
            .delete_many(InvoiceFilter {
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

    async fn delete_many(&self, filter: InvoiceFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting lightning invoices");

        let n_deleted = self.store.invoice.delete_many(filter.clone()).await?;

        info!(
            ?filter,
            n_deleted, "Lightning invoices deleted successfully"
        );
        Ok(n_deleted)
    }
}
