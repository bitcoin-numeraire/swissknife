use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    application::{
        entities::{AppStore, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::{
        invoices::entities::{Invoice, InvoiceFilter, InvoiceOrderBy, InvoiceStatus},
        lightning::entities::{LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent},
        payments::entities::PaymentStatus,
    },
};

use super::LnEventsUseCases;

pub struct LnEventsService {
    store: AppStore,
}

impl LnEventsService {
    pub fn new(store: AppStore) -> Self {
        LnEventsService { store }
    }
}

#[async_trait]
impl LnEventsUseCases for LnEventsService {
    // TODO: Remove this when we can simply listen to the latest events.
    async fn latest_settled_invoice(&self) -> Result<Option<Invoice>, ApplicationError> {
        debug!("Fetching latest settled invoice...");

        let invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                status: Some(InvoiceStatus::Settled),
                ledger: Some(Ledger::Lightning),
                limit: Some(1),
                order_by: InvoiceOrderBy::PaymentTime,
                ..Default::default()
            })
            .await?;

        Ok(invoices.into_iter().next())
    }

    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError> {
        debug!(?event, "Processing incoming Lightning payment...");

        let invoice_option = if let Some(ref hash) = event.payment_hash {
            self.store.invoice.find_by_payment_hash(hash).await?
        } else if let Some(id) = event.id {
            self.store.invoice.find(id).await?
        } else {
            None
        };

        if let Some(mut invoice) = invoice_option {
            invoice.fee_msat = Some(event.fee_msat);
            invoice.payment_time = Some(event.payment_time);

            // TODO: Amount paid can actually be different from the amount of the invoice, might need a new field (amount_received_msat).
            // we can just update the amount_msat field for now.
            invoice.amount_received_msat = Some(event.amount_received_msat);

            invoice = self.store.invoice.update(None, invoice).await?;

            info!(
                id = %invoice.id,
                "Incoming Lightning payment processed successfully"
            );
            return Ok(());
        }

        return Err(DataError::NotFound("Lightning invoice not found.".into()).into());
    }

    async fn outgoing_payment(&self, event: LnPaySuccessEvent) -> Result<(), ApplicationError> {
        debug!(?event, "Processing outgoing Lightning payment...");

        let payment_option = self
            .store
            .payment
            .find_by_payment_hash(&event.payment_hash)
            .await?;

        if let Some(mut payment_retrieved) = payment_option {
            if payment_retrieved.status == PaymentStatus::Settled {
                debug!(
                    id = %payment_retrieved.id,
                    "Lightning payment already settled"
                );
                return Ok(());
            }

            payment_retrieved.status = PaymentStatus::Settled;
            payment_retrieved.payment_time = Some(event.payment_time);
            payment_retrieved.payment_preimage = Some(event.payment_preimage);
            payment_retrieved.amount_msat = event.amount_msat;
            payment_retrieved.fee_msat = Some(event.fees_msat);

            let payment = self.store.payment.update(payment_retrieved).await?;

            info!(
                id = %payment.id,
                payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully"
            );
            return Ok(());
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }

    async fn failed_payment(&self, event: LnPayFailureEvent) -> Result<(), ApplicationError> {
        debug!(?event, "Processing failed outgoing Lightning payment");

        let payment_option = self
            .store
            .payment
            .find_by_payment_hash(&event.payment_hash)
            .await?;

        if let Some(mut payment_retrieved) = payment_option {
            if payment_retrieved.status == PaymentStatus::Failed {
                debug!(
                    id = %payment_retrieved.id,
                    "Lightning payment already failed"
                );
                return Ok(());
            }

            payment_retrieved.status = PaymentStatus::Failed;
            payment_retrieved.error = Some(event.reason);

            let payment = self.store.payment.update(payment_retrieved).await?;

            info!(
                id = %payment.id,
                payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully"
            );
            return Ok(());
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }
}
