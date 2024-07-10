use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        dtos::LightningProvider,
        entities::{AppStore, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::invoices::entities::{Invoice, InvoiceFilter, InvoiceStatus},
};

use std::sync::Arc;

use crate::infra::lightning::LnClient;

use super::InvoiceUseCases;

const DEFAULT_INVOICE_DESCRIPTION: &str = "Numeraire Swissknife Invoice";

pub struct InvoiceService {
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
    invoice_expiry: u32,
    ln_provider: LightningProvider,
}

impl InvoiceService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        invoice_expiry: u32,
        ln_provider: LightningProvider,
    ) -> Self {
        InvoiceService {
            store,
            ln_client,
            invoice_expiry,
            ln_provider,
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
        debug!(%user_id, "Generating invoice");

        let mut invoice = self
            .ln_client
            .invoice(
                amount,
                description.unwrap_or(DEFAULT_INVOICE_DESCRIPTION.to_string()),
                expiry.unwrap_or(self.invoice_expiry),
            )
            .await?;
        invoice.user_id.clone_from(&user_id);

        let invoice = self.store.invoice.insert(None, invoice).await?;

        info!(
            id = invoice.id.to_string(),
            "Invoice generated successfully"
        );
        Ok(invoice)
    }

    async fn get(&self, id: Uuid) -> Result<Invoice, ApplicationError> {
        trace!(
            %id,
            "Fetching invoice"
        );

        let lightning_invoice = self
            .store
            .invoice
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        debug!(
            %id, "Invoice fetched successfully"
        );
        Ok(lightning_invoice)
    }

    async fn list(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, ApplicationError> {
        trace!(?filter, "Listing invoices");

        let lightning_invoices = self.store.invoice.find_many(filter.clone()).await?;

        debug!(?filter, "Invoices listed successfully");
        Ok(lightning_invoices)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting invoice");

        let n_deleted = self
            .store
            .invoice
            .delete_many(InvoiceFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Invoice not found.".to_string()).into());
        }

        info!(%id, "Invoice deleted successfully");
        Ok(())
    }

    async fn delete_many(&self, filter: InvoiceFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting invoices");

        let n_deleted = self.store.invoice.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Invoices deleted successfully");
        Ok(n_deleted)
    }

    // TODO: Move to Lightning node
    async fn sync(&self) -> Result<u32, ApplicationError> {
        trace!(ln_provider = %self.ln_provider, "Syncing invoices...");

        if self.ln_provider != LightningProvider::ClnRest {
            debug!("Lightning provider does not need initial syncing");
            return Ok(0);
        }

        let mut n_synced = 0;

        let pending_invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                status: Some(InvoiceStatus::Pending),
                ledger: Some(Ledger::Lightning),
                ..Default::default()
            })
            .await?;

        // We have to also check the expired invoices because they can become expired while the app is down and the payment received
        // Ideally the expired invoices should be cleaned, not to have too many to sync on startup
        let expired_invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                status: Some(InvoiceStatus::Expired),
                ledger: Some(Ledger::Lightning),
                ..Default::default()
            })
            .await?;

        let invoices = pending_invoices
            .into_iter()
            .chain(expired_invoices.into_iter());

        for invoice in invoices {
            let payment_hash = invoice
                .ln_invoice
                .as_ref()
                .expect("Invoice should have a lightning field")
                .payment_hash
                .clone();

            if let Some(node_invoice) = self.ln_client.invoice_by_hash(payment_hash).await? {
                if node_invoice.status == InvoiceStatus::Settled {
                    let mut updated_invoice = invoice.clone();

                    updated_invoice.status = node_invoice.status;
                    updated_invoice.payment_time = node_invoice.payment_time;
                    updated_invoice.amount_msat = node_invoice.amount_msat;

                    self.store.invoice.update(None, updated_invoice).await?;
                    n_synced += 1;
                }
            }
        }

        info!(%n_synced, "Invoices synced successfully");
        Ok(n_synced)
    }
}
