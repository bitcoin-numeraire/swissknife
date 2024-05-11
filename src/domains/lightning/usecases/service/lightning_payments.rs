use async_trait::async_trait;
use breez_sdk_core::{parse, InputType, LNInvoice, LnUrlPayRequestData};
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError, LightningError},
    domains::{
        lightning::{
            entities::{LightningPayment, LightningPaymentStatus},
            usecases::LightningPaymentsUseCases,
        },
        users::entities::{AuthUser, Permission},
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
        user: AuthUser,
        invoice: LNInvoice,
        amount_msat: Option<u64>,
        send_from_node: bool,
    ) -> Result<LightningPayment, ApplicationError> {
        let specified_amount = invoice.amount_msat.or(amount_msat);
        if specified_amount == Some(0) {
            return Err(
                DataError::Validation("Amount must be greater than zero.".to_string()).into(),
            );
        }

        let txn = if send_from_node {
            warn!(user_id = user.sub, "Sending payment from node");
            None
        } else {
            Some(self.store.begin().await?)
        };

        // If we are in a transaction, we check in DB as we are paying from the user's balance
        let balance = match txn {
            Some(_) => {
                self.store
                    .get_balance(txn.as_ref(), &user.sub)
                    .await?
                    .available_msat as u64
            }
            None => self.lightning_client.node_info()?.max_payable_msat,
        };

        if let Some(amount) = specified_amount {
            if balance < amount {
                return Err(DataError::InsufficientFunds.into());
            }

            let pending_payment = self
                .store
                .insert_payment(
                    txn.as_ref(),
                    LightningPayment {
                        user_id: user.sub,
                        amount_msat: amount,
                        status: LightningPaymentStatus::PENDING,
                        payment_hash: Some(invoice.payment_hash),
                        ..Default::default()
                    },
                )
                .await?;

            // Commit transaction if available
            if let Some(t) = txn {
                t.commit()
                    .await
                    .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
            }

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
        user: AuthUser,
        data: LnUrlPayRequestData,
        amount_msat: Option<u64>,
        comment: Option<String>,
        send_from_node: bool,
    ) -> Result<LightningPayment, ApplicationError> {
        let amount = LightningService::validate_amount(amount_msat)?;

        let txn = if send_from_node {
            None
        } else {
            Some(self.store.begin().await?)
        };

        // If we are in a transaction, we check in DB as we are paying from the user's balance
        let balance = match txn {
            Some(_) => {
                self.store
                    .get_balance(txn.as_ref(), &user.sub)
                    .await?
                    .available_msat as u64
            }
            None => self.lightning_client.node_info()?.max_payable_msat,
        };

        if balance <= amount {
            return Err(DataError::InsufficientFunds.into());
        }

        let pending_payment = self
            .store
            .insert_payment(
                txn.as_ref(),
                LightningPayment {
                    user_id: user.sub,
                    amount_msat: amount,
                    status: LightningPaymentStatus::PENDING,
                    lightning_address: data.ln_address.clone(),
                    ..Default::default()
                },
            )
            .await?;

        // Commit transaction if available
        if let Some(t) = txn {
            t.commit()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        }

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
    async fn get_payment(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningPayment, ApplicationError> {
        trace!(user_id = user.sub, "Fetching lightning payment");

        let lightning_payment = self
            .store
            .find_payment(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning payment not found.".to_string()))?;

        if lightning_payment.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAccounts)?;
        }

        debug!(
            user_id = user.sub,
            id = id.to_string(),
            "Lightning payment fetched successfully"
        );
        Ok(lightning_payment)
    }

    async fn list_payments(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningPayment>, ApplicationError> {
        trace!(
            user_id = user.sub,
            limit,
            offset,
            "Listing lightning payments"
        );

        let lightning_payments = if user.has_permission(Permission::ReadLightningAccounts) {
            // The user has permission to view all addresses
            self.store.find_all_payments(None, limit, offset).await?
        } else {
            // The user can only view their own payments
            self.store
                .find_all_payments(Some(user.sub.clone()), limit, offset)
                .await?
        };

        debug!(user_id = user.sub, "Lightning payments listed successfully");
        Ok(lightning_payments)
    }

    async fn pay(
        &self,
        user: AuthUser,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
    ) -> Result<LightningPayment, ApplicationError> {
        debug!(user_id = user.sub, input, "Sending payment");

        // If user has permission, we do not check the balance but the node balance
        let can_send_from_node = user.has_permission(Permission::SendLightningPayment);

        let input_type = parse(&input)
            .await
            .map_err(|e| DataError::Validation(e.to_string()))?;

        let payment = match input_type {
            InputType::Bolt11 { invoice } => {
                self.send_bolt11(user.clone(), invoice, amount_msat, can_send_from_node)
                    .await
            }
            InputType::LnUrlPay { data } => {
                self.send_lnurl_pay(user.clone(), data, amount_msat, comment, can_send_from_node)
                    .await
            }
            InputType::LnUrlError { data } => Err(DataError::Validation(data.reason).into()),
            _ => Err(DataError::Validation("Unsupported payment format".to_string()).into()),
        }?;

        info!(
            user_id = user.sub,
            input,
            amount_msat,
            status = payment.status.to_string(),
            "Payment processed successfully"
        );

        Ok(payment)
    }
}
