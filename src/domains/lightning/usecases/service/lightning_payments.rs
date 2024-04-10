use async_trait::async_trait;
use breez_sdk_core::{Payment, PaymentFailedData, PaymentStatus, PaymentType};
use tracing::{debug, info, trace};

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::lightning::{
        entities::{LightningInvoice, LightningPayment},
        store::LightningStore,
        usecases::LightningPaymentsUseCases,
    },
};

pub struct LightningPaymentsProcessor {
    pub store: LightningStore,
}

impl LightningPaymentsProcessor {
    pub fn new(store: LightningStore) -> Self {
        LightningPaymentsProcessor { store }
    }
}

#[async_trait]
impl LightningPaymentsUseCases for LightningPaymentsProcessor {
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

        let invoice_option = self.store.invoice_repo.get_by_hash(&payment_hash).await?;

        if let Some(mut invoice) = invoice_option {
            if invoice.status == "PAID".to_string() {
                debug!(payment_hash, "Lightning invoice is already paid.");
                return Ok(invoice);
            }

            invoice.fee_msat = Some(payment.fee_msat as i64);
            invoice.status = "PAID".to_string();
            invoice.payment_time = Some(payment.payment_time);

            invoice = self.store.invoice_repo.update(invoice).await?;

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
        let payment_hash = payment_success.id;
        trace!(payment_hash, "Processing outgoing lightning payment");

        if payment_success.status != PaymentStatus::Complete {
            return Err(DataError::Validation("Payment is not Complete.".into()).into());
        }

        if payment_success.payment_type != PaymentType::Sent {
            return Err(DataError::Validation("Payment is not Sent.".into()).into());
        }

        let payment_option = self.store.payment_repo.get_by_hash(&payment_hash).await?;

        if let Some(mut payment) = payment_option {
            if payment.status == "PAID".to_string() {
                debug!(payment_hash, "Lightning invoice is already paid.");
                return Ok(payment);
            }

            payment.fee_msat = Some(payment_success.fee_msat as i64);
            payment.status = "PAID".to_string();
            payment.payment_time = Some(payment_success.payment_time);

            payment = self.store.payment_repo.update(payment).await?;

            info!(
                payment_hash,
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
        if payment_failed.invoice.is_none() {
            return Err(
                DataError::Validation("Failed payment is missing an Invoice.".into()).into(),
            );
        }

        let invoice = payment_failed.invoice.unwrap();
        trace!(
            payment_hash = invoice.payment_hash,
            "Processing outgoing failed lightning payment"
        );

        let payment_option = self
            .store
            .payment_repo
            .get_by_hash(&invoice.payment_hash)
            .await?;

        if let Some(mut payment) = payment_option {
            if payment.status == "PAID".to_string() {
                debug!(
                    payment_hash = invoice.payment_hash,
                    "Lightning invoice is already paid."
                );
                return Ok(payment);
            }

            payment.status = "FAILED".to_string();
            payment.payment_time = Some(invoice.timestamp as i64);
            payment.error = Some(payment_failed.error);
            payment = self.store.payment_repo.update(payment).await?;

            info!(
                payment_hash = invoice.payment_hash,
                username = payment.lightning_address,
                "Outgoing Lightning payment processed successfully"
            );
            return Ok(payment);
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }
}
