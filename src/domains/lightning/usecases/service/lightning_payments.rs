use async_trait::async_trait;
use breez_sdk_core::{Payment, PaymentStatus, PaymentType};
use tracing::{debug, info, trace};

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::lightning::{
        entities::LightningInvoice, store::LightningInvoiceRepository,
        usecases::LightningPaymentsUseCases,
    },
};

pub struct LightningPaymentsProcessor {
    pub invoice_repo: Box<dyn LightningInvoiceRepository>,
}

impl LightningPaymentsProcessor {
    pub fn new(invoice_repo: Box<dyn LightningInvoiceRepository>) -> Self {
        LightningPaymentsProcessor { invoice_repo }
    }
}

#[async_trait]
impl LightningPaymentsUseCases for LightningPaymentsProcessor {
    async fn process_incoming_payment(
        &self,
        payment: Payment,
    ) -> Result<LightningInvoice, ApplicationError> {
        let payment_hash = payment.id;
        trace!(payment_hash, "Processing lightning payment");

        if payment.status != PaymentStatus::Complete {
            return Err(DataError::Validation("Payment is not Complete.".into()).into());
        }

        if payment.payment_type != PaymentType::Received {
            return Err(DataError::Validation("Payment is not Received.".into()).into());
        }

        let invoice_option = self.invoice_repo.get_by_hash(&payment_hash).await?;

        if let Some(mut invoice) = invoice_option {
            if invoice.status == "PAID".to_string() {
                debug!(payment_hash, "Lightning invoice is already paid.");
                return Ok(invoice);
            }

            invoice.fee_msat = Some(payment.fee_msat as i64);
            invoice.status = "PAID".to_string();
            invoice.payment_time = Some(payment.payment_time);

            invoice = self.invoice_repo.update(invoice).await?;

            info!(payment_hash, "Lightning payment processed successfully");
            return Ok(invoice);
        }

        return Err(DataError::NotFound("Lightning invoice not found.".into()).into());
    }
}
