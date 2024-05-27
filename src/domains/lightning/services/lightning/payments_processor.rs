use async_trait::async_trait;
use breez_sdk_core::{Payment, PaymentDetails, PaymentFailedData, PaymentStatus, PaymentType};
use chrono::{TimeZone, Utc};
use tracing::{info, trace};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::lightning::{
        adapters::LightningRepository,
        entities::{LightningInvoice, LightningPayment, LightningPaymentStatus},
        services::PaymentsProcessorUseCases,
    },
};

pub struct BreezPaymentsProcessor {
    pub store: Box<dyn LightningRepository>,
}

impl BreezPaymentsProcessor {
    pub fn new(store: Box<dyn LightningRepository>) -> Self {
        BreezPaymentsProcessor { store }
    }
}

#[async_trait]
impl PaymentsProcessorUseCases for BreezPaymentsProcessor {
    async fn process_incoming_payment(
        &self,
        payment: Payment,
    ) -> Result<LightningInvoice, ApplicationError> {
        let payment_hash = payment.id;
        trace!(payment_hash, "Processing incoming lightning payment");

        if payment.status != PaymentStatus::Complete {
            return Err(DataError::Validation("Payment is not Complete.".into()).into());
        }

        if payment.payment_type != PaymentType::Received {
            return Err(DataError::Validation("Payment is not Received.".into()).into());
        }

        let invoice_option = self
            .store
            .find_invoice_by_payment_hash(&payment_hash)
            .await?;

        if let Some(mut invoice) = invoice_option {
            invoice.fee_msat = Some(payment.fee_msat);
            invoice.payment_time = Some(Utc.timestamp_opt(payment.payment_time, 0).unwrap());
            // This is needed because the amount_msat is not always the same as the invoice amount because of fees and fees are not part of the event
            // Until this is fixed: https://github.com/breez/breez-sdk/issues/982
            invoice.amount_msat = Some(payment.amount_msat);

            invoice = self.store.update_invoice(invoice).await?;

            info!(
                payment_hash,
                "Incoming Lightning payment processed successfully"
            );
            return Ok(invoice);
        }

        return Err(DataError::NotFound("Lightning invoice not found.".into()).into());
    }

    async fn process_outgoing_payment(
        &self,
        payment_success: Payment,
    ) -> Result<LightningPayment, ApplicationError> {
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

        if payment_success.status != PaymentStatus::Complete {
            return Err(DataError::Validation("Payment is not Complete.".into()).into());
        }

        if payment_success.payment_type != PaymentType::Sent {
            return Err(DataError::Validation("Payment is not Sent.".into()).into());
        }

        let payment_option = self.store.find_payment(payment_id).await?;

        if let Some(payment_retrieved) = payment_option {
            // We overwrite the payment with the new one at the correct status
            let mut payment: LightningPayment = payment_success.clone().into();
            payment.id = payment_retrieved.id;
            payment.status = LightningPaymentStatus::SETTLED;

            let payment = self.store.update_payment(payment).await?;

            info!(
                %payment_id,
                payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully"
            );
            return Ok(payment);
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }

    async fn process_failed_payment(
        &self,
        payment_failed: PaymentFailedData,
    ) -> Result<LightningPayment, ApplicationError> {
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

        let payment_option = self.store.find_payment(payment_id).await?;

        if let Some(mut payment) = payment_option {
            payment.status = LightningPaymentStatus::FAILED;
            payment.payment_time = Some(Utc.timestamp_opt(invoice.timestamp as i64, 0).unwrap());
            payment.error = Some(payment_failed.error);
            payment.payment_hash = Some(invoice.payment_hash);

            payment = self.store.update_payment(payment).await?;

            info!(
                %payment_id,
                payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully"
            );
            return Ok(payment);
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }
}