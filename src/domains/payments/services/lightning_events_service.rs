use async_trait::async_trait;
use breez_sdk_core::{Payment as BreezPayment, PaymentDetails, PaymentFailedData};
use chrono::{TimeZone, Utc};
use tracing::{info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::payments::entities::{Payment, PaymentStatus},
};

use super::LightningEventsUseCases;

pub struct LightningEventsService {
    store: AppStore,
}

impl LightningEventsService {
    pub fn new(store: AppStore) -> Self {
        LightningEventsService { store }
    }
}

#[async_trait]
impl LightningEventsUseCases for LightningEventsService {
    async fn process_incoming_payment(
        &self,
        payment: BreezPayment,
    ) -> Result<(), ApplicationError> {
        let payment_hash = payment.id;
        trace!(payment_hash, "Processing incoming lightning payment");

        let invoice_option = self
            .store
            .invoice
            .find_invoice_by_payment_hash(&payment_hash)
            .await?;

        if let Some(mut invoice) = invoice_option {
            invoice.fee_msat = Some(payment.fee_msat);
            invoice.payment_time = Some(Utc.timestamp_opt(payment.payment_time, 0).unwrap());
            // This is needed because the amount_msat is not always the same as the invoice amount because of fees and fees are not part of the event
            // Until this is fixed: https://github.com/breez/breez-sdk/issues/982
            invoice.amount_msat = Some(payment.amount_msat);

            invoice = self.store.invoice.update_invoice(None, invoice).await?;

            info!(
                id = invoice.id.to_string(),
                payment_hash, "Incoming Lightning payment processed successfully"
            );
            return Ok(());
        }

        return Err(DataError::NotFound("Lightning invoice not found.".into()).into());
    }

    async fn process_outgoing_payment(
        &self,
        payment_success: BreezPayment,
    ) -> Result<(), ApplicationError> {
        let payment_id = match payment_success.details.clone() {
            PaymentDetails::Ln { data } => {
                Uuid::parse_str(&data.label).map_err(|e| DataError::Validation(e.to_string()))
            }
            _ => Err(DataError::NotFound("Missing lightning payment details".into()).into()),
        }?;
        trace!(
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

    async fn process_failed_payment(
        &self,
        payment_failed: PaymentFailedData,
    ) -> Result<(), ApplicationError> {
        let payment_id = match payment_failed.label {
            Some(label) => {
                Uuid::parse_str(&label).map_err(|e| DataError::Validation(e.to_string()))
            }
            None => Err(DataError::NotFound("Missing lightning payment label".into()).into()),
        }?;
        trace!(
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
