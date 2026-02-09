use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, info, trace};
use uuid::Uuid;

use std::sync::Arc;

use crate::{
    application::{
        entities::{AppStore, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::event::{EventUseCases, LnInvoicePaidEvent},
    infra::lightning::LnClient,
};

use super::{Invoice, InvoiceFilter, InvoiceStatus, InvoiceUseCases};

const DEFAULT_INVOICE_DESCRIPTION: &str = "Numeraire Invoice";

pub struct InvoiceService {
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
    invoice_expiry: u32,
    events: Arc<dyn EventUseCases>,
}

impl InvoiceService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        invoice_expiry: u32,
        events: Arc<dyn EventUseCases>,
    ) -> Self {
        InvoiceService {
            store,
            ln_client,
            invoice_expiry,
            events,
        }
    }
}

#[async_trait]
impl InvoiceUseCases for InvoiceService {
    async fn invoice(
        &self,
        wallet_id: Uuid,
        amount: u64,
        description: Option<String>,
        expiry: Option<u32>,
    ) -> Result<Invoice, ApplicationError> {
        debug!(%wallet_id, "Generating invoice");

        let invoice_id = Uuid::new_v4();
        let mut invoice = self
            .ln_client
            .invoice(
                amount,
                description.unwrap_or(DEFAULT_INVOICE_DESCRIPTION.to_string()),
                invoice_id.to_string(),
                expiry.unwrap_or(self.invoice_expiry),
                false,
            )
            .await?;
        invoice.id = invoice_id;
        invoice.wallet_id.clone_from(&wallet_id);

        let invoice = self.store.invoice.insert(invoice).await?;

        info!(id = %invoice.id, "Invoice generated successfully");
        Ok(invoice)
    }

    async fn get(&self, id: Uuid) -> Result<Invoice, ApplicationError> {
        trace!(%id, "Fetching invoice");

        let invoice = self
            .store
            .invoice
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?;

        debug!(%id, "Invoice fetched successfully");
        Ok(invoice)
    }

    async fn list(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, ApplicationError> {
        trace!(?filter, "Listing invoices");

        let invoices = self.store.invoice.find_many(filter.clone()).await?;

        debug!(?filter, "Invoices listed successfully");
        Ok(invoices)
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

    async fn sync(&self) -> Result<u32, ApplicationError> {
        trace!("Synchronizing pending and expired invoices...");

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

        let invoices = pending_invoices.into_iter().chain(expired_invoices);

        let mut synced = 0;

        for invoice in invoices {
            let Some(ln_invoice) = invoice.ln_invoice.as_ref() else {
                debug!(invoice_id = %invoice.id, "Missing lightning invoice details; skipping sync");
                continue;
            };
            let payment_hash = ln_invoice.payment_hash.clone();
            let Some(node_invoice) = self.ln_client.invoice_by_hash(payment_hash.clone()).await? else {
                continue;
            };
            if node_invoice.status != InvoiceStatus::Settled {
                continue;
            }

            let payment_time = node_invoice.payment_time.unwrap_or_else(Utc::now);
            let event = LnInvoicePaidEvent {
                payment_hash,
                amount_received_msat: node_invoice.amount_received_msat.unwrap_or_default(),
                fee_msat: node_invoice.fee_msat.unwrap_or_default(),
                payment_time,
            };

            self.events.invoice_paid(event).await?;
            synced += 1;
        }

        debug!(synced, "Pending and expired invoices synchronized successfully");
        Ok(synced)
    }
}
