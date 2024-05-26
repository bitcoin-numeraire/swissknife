use async_trait::async_trait;
use breez_sdk_core::{parse, InputType, LNInvoice, LnUrlPayRequestData};
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        dtos::{LightningPaymentFilter, SendPaymentRequest},
        errors::{ApplicationError, DataError, DatabaseError, LightningError},
    },
    domains::lightning::{
        entities::{LightningPayment, LightningPaymentStatus},
        services::LightningPaymentsUseCases,
    },
};

use super::LightningService;

impl LightningService {
    pub(crate) fn validate_amount(amount_msat: Option<u64>) -> Result<u64, ApplicationError> {
        let amount = amount_msat.unwrap_or_default();
        if amount == 0 {
            return Err(
                DataError::Validation("Amount must be greater than zero".to_string()).into(),
            );
        }

        Ok(amount)
    }

    async fn send_bolt11(
        &self,
        user_id: String,
        invoice: LNInvoice,
        amount_msat: Option<u64>,
    ) -> Result<LightningPayment, ApplicationError> {
        let specified_amount = invoice.amount_msat.or(amount_msat);
        if specified_amount == Some(0) {
            return Err(
                DataError::Validation("Amount must be greater than zero.".to_string()).into(),
            );
        }

        let txn = self.store.begin().await?;

        let balance = self
            .store
            .get_balance(Some(&txn), &user_id)
            .await?
            .available_msat as u64;

        if let Some(amount) = specified_amount {
            if balance < amount {
                return Err(DataError::InsufficientFunds.into());
            }

            let pending_payment = self
                .store
                .insert_payment(
                    Some(&txn),
                    LightningPayment {
                        user_id,
                        amount_msat: amount,
                        status: LightningPaymentStatus::PENDING,
                        payment_hash: Some(invoice.payment_hash),
                        description: invoice.description,
                        ..Default::default()
                    },
                )
                .await?;

            txn.commit()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

            let result = self
                .lightning_client
                .send_payment(
                    invoice.bolt11.clone(),
                    if invoice.amount_msat.is_some() {
                        None
                    } else {
                        Some(amount)
                    },
                    pending_payment.id,
                )
                .await;

            self.handle_processed_payment(pending_payment, result).await
        } else {
            Err(DataError::Validation(
                "Amount must be defined for zero-amount invoices.".to_string(),
            )
            .into())
        }
    }

    async fn send_lnurl_pay(
        &self,
        user_id: String,
        data: LnUrlPayRequestData,
        amount_msat: Option<u64>,
        comment: Option<String>,
    ) -> Result<LightningPayment, ApplicationError> {
        let amount = LightningService::validate_amount(amount_msat)?;

        let txn = self.store.begin().await?;

        let balance = self
            .store
            .get_balance(Some(&txn), &user_id)
            .await?
            .available_msat as u64;

        if balance <= amount {
            return Err(DataError::InsufficientFunds.into());
        }

        let pending_payment = self
            .store
            .insert_payment(
                Some(&txn),
                LightningPayment {
                    user_id,
                    amount_msat: amount,
                    status: LightningPaymentStatus::PENDING,
                    lightning_address: data.ln_address.clone(),
                    description: comment.clone(),
                    ..Default::default()
                },
            )
            .await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let result = self
            .lightning_client
            .lnurl_pay(data, amount, comment, pending_payment.id)
            .await;

        self.handle_processed_payment(pending_payment, result).await
    }

    async fn handle_processed_payment(
        &self,
        mut pending_payment: LightningPayment,
        result: Result<LightningPayment, LightningError>,
    ) -> Result<LightningPayment, ApplicationError> {
        match result {
            Ok(mut payment) => {
                payment.id = pending_payment.id;
                payment.status = match payment.error {
                    // There is a case where the payment fails but still has a payment_hash and a payment is returned instead of an error
                    Some(_) => LightningPaymentStatus::FAILED,
                    None => LightningPaymentStatus::SETTLED,
                };
                let payment: LightningPayment = self.store.update_payment(payment).await?;
                Ok(payment)
            }
            Err(error) => {
                pending_payment.status = LightningPaymentStatus::FAILED;
                pending_payment.error = Some(error.to_string());
                let payment: LightningPayment = self.store.update_payment(pending_payment).await?;
                Ok(payment)
            }
        }
    }
}

#[async_trait]
impl LightningPaymentsUseCases for LightningService {
    async fn get_payment(&self, id: Uuid) -> Result<LightningPayment, ApplicationError> {
        trace!(%id, "Fetching lightning payment");

        let lightning_payment = self
            .store
            .find_payment(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning payment not found.".to_string()))?;

        debug!(
            %id,
            "Lightning payment fetched successfully"
        );
        Ok(lightning_payment)
    }

    async fn list_payments(
        &self,
        filter: LightningPaymentFilter,
    ) -> Result<Vec<LightningPayment>, ApplicationError> {
        trace!(?filter, "Listing lightning payments");

        let lightning_payments = self.store.find_payments(filter.clone()).await?;

        debug!(?filter, "Lightning payments listed successfully");
        Ok(lightning_payments)
    }

    async fn pay(&self, req: SendPaymentRequest) -> Result<LightningPayment, ApplicationError> {
        debug!(?req, "Sending payment");

        let input_type = parse(&req.input)
            .await
            .map_err(|e| DataError::Validation(e.to_string()))?;

        let payment = match input_type {
            InputType::Bolt11 { invoice } => {
                self.send_bolt11(req.user_id, invoice, req.amount_msat)
                    .await
            }
            InputType::LnUrlPay { data } => {
                self.send_lnurl_pay(req.user_id, data, req.amount_msat, req.comment)
                    .await
            }
            InputType::LnUrlError { data } => Err(DataError::Validation(data.reason).into()),
            _ => Err(DataError::Validation("Unsupported payment format".to_string()).into()),
        }?;

        info!(
            id = payment.id.to_string(),
            "Payment processed successfully"
        );

        Ok(payment)
    }

    async fn delete_payment(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting lightning payment");

        let n_deleted = self
            .store
            .delete_payments(LightningPaymentFilter {
                id: Some(id),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Lightning payment not found.".to_string()).into());
        }

        info!(%id, "Lightning payments deleted successfully");
        Ok(())
    }

    async fn delete_payments(
        &self,
        filter: LightningPaymentFilter,
    ) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting lightning payments");

        let n_deleted = self.store.delete_payments(filter.clone()).await?;

        info!(
            ?filter,
            n_deleted, "Lightning payments deleted successfully"
        );
        Ok(n_deleted)
    }
}
