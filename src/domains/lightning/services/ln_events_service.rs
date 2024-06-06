use async_trait::async_trait;
use breez_sdk_core::{Payment as BreezPayment, PaymentDetails, PaymentFailedData};
use chrono::{TimeZone, Utc};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, Ledger, PaginationFilter},
        errors::{ApplicationError, DataError},
    },
    domains::{
        invoices::entities::{Invoice, InvoiceFilter, InvoiceStatus},
        lightning::entities::LnInvoicePaidEvent,
        payments::entities::{Payment, PaymentStatus},
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
                status: Some(InvoiceStatus::SETTLED),
                ledger: Some(Ledger::LIGHTNING),
                pagination: PaginationFilter {
                    limit: Some(1),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;

        Ok(invoices.into_iter().next())
    }

    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError> {
        debug!(?event, "Processing incoming lightning payment...");

        let invoice_option = self
            .store
            .invoice
            .find_by_payment_hash(&event.payment_hash)
            .await?;

        if let Some(mut invoice) = invoice_option {
            invoice.fee_msat = Some(event.fee_msat);
            invoice.payment_time = Some(event.payment_time);

            // TODO: Amount paid can actually be different from the amount of the invoice, might need a new field (amount_received_msat).
            // we can just update the amount_msat field for now.
            invoice.amount_msat = Some(event.amount_msat);

            invoice = self.store.invoice.update(None, invoice).await?;

            info!(
                id = %invoice.id,
                "Incoming Lightning payment processed successfully"
            );
            return Ok(());
        }

        return Err(DataError::NotFound("Lightning invoice not found.".into()).into());
    }

    async fn outgoing_payment(
        &self,
        payment_success: BreezPayment,
    ) -> Result<(), ApplicationError> {
        let payment_id = match payment_success.details.clone() {
            PaymentDetails::Ln { data } => {
                Uuid::parse_str(&data.label).map_err(|e| DataError::Validation(e.to_string()))
            }
            _ => Err(DataError::NotFound("Missing lightning payment details".into()).into()),
        }?;
        debug!(
            %payment_id,
            "Processing outgoing lightning payment"
        );

        let payment_option = self.store.payment.find(payment_id).await?;

        if let Some(payment_retrieved) = payment_option {
            // We overwrite the payment with the new one at the correct status
            let mut payment: Payment = payment_success.clone().into();
            payment.id = payment_retrieved.id;
            payment.status = PaymentStatus::SETTLED;

            let payment = self.store.payment.update(payment).await?;

            info!(
                %payment_id,
                payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully"
            );
            return Ok(());
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }

    async fn failed_payment(
        &self,
        payment_failed: PaymentFailedData,
    ) -> Result<(), ApplicationError> {
        let payment_id = match payment_failed.label {
            Some(label) => {
                Uuid::parse_str(&label).map_err(|e| DataError::Validation(e.to_string()))
            }
            None => Err(DataError::NotFound("Missing lightning payment label".into()).into()),
        }?;
        debug!(
            %payment_id,
            "Processing outgoing failed lightning payment"
        );

        let invoice = match payment_failed.invoice {
            Some(invoice) => invoice,
            None => {
                return Err(
                    DataError::Validation("Failed payment is missing an Invoice.".into()).into(),
                );
            }
        };

        let payment_option = self.store.payment.find(payment_id).await?;

        if let Some(mut payment) = payment_option {
            payment.status = PaymentStatus::FAILED;
            payment.payment_time = Some(Utc.timestamp_opt(invoice.timestamp as i64, 0).unwrap());
            payment.error = Some(payment_failed.error);
            payment.payment_hash = Some(invoice.payment_hash);

            payment = self.store.payment.update(payment).await?;

            info!(
                %payment_id,
                payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully"
            );
            return Ok(());
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }
}
