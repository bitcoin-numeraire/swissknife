use std::sync::Arc;

use async_trait::async_trait;
use breez_sdk_core::{parse, InputType, LNInvoice, LnUrlPayRequestData};
use chrono::Utc;
use serde_bolt::bitcoin::Network;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        dtos::SendPaymentRequest,
        entities::AppStore,
        errors::{ApplicationError, DataError, DatabaseError, LightningError},
    },
    domains::{
        invoices::entities::{Invoice, InvoiceStatus, InvoiceType},
        payments::entities::{Payment, PaymentFilter, PaymentStatus, PaymentType},
    },
    infra::lightning::LightningClient,
};

use super::PaymentsUseCases;

const DEFAULT_INTERNAL_INVOICE_DESCRIPTION: &str = "Numeraire Swissknife Invoice";
const DEFAULT_INTERNAL_PAYMENT_DESCRIPTION: &str = "Payment to Numeraire Swissknife";

pub struct PaymentsService {
    domain: String,
    store: AppStore,
    lightning_client: Arc<dyn LightningClient>,
    fee_buffer: f64,
}

impl PaymentsService {
    pub fn new(
        store: AppStore,
        lightning_client: Arc<dyn LightningClient>,
        domain: String,
        fee_buffer: f64,
    ) -> Self {
        PaymentsService {
            store,
            lightning_client,
            domain,
            fee_buffer,
        }
    }
}

impl PaymentsService {
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
    ) -> Result<Payment, ApplicationError> {
        let specified_amount = invoice.amount_msat.or(amount_msat);
        if specified_amount == Some(0) {
            return Err(
                DataError::Validation("Amount must be greater than zero.".to_string()).into(),
            );
        }

        if let Some(amount) = specified_amount {
            // Check if internal payment
            let invoice_opt = self
                .store
                .invoice
                .find_by_payment_hash(&invoice.payment_hash)
                .await?;
            if let Some(mut retrieved_invoice) = invoice_opt {
                if retrieved_invoice.user_id == user_id {
                    return Err(
                        DataError::Validation("Cannot pay for own invoice.".to_string()).into(),
                    );
                }

                match retrieved_invoice.status {
                    InvoiceStatus::SETTLED => {
                        return Err(DataError::Validation(
                            "Invoice has already been paid.".to_string(),
                        )
                        .into());
                    }
                    InvoiceStatus::EXPIRED => {
                        return Err(DataError::Validation("Invoice is expired.".to_string()).into());
                    }
                    InvoiceStatus::PENDING => {
                        // Internal payment
                        let txn = self.store.lightning.begin().await?;

                        let internal_payment = self
                            .insert_payment(
                                Payment {
                                    user_id,
                                    amount_msat: amount,
                                    status: PaymentStatus::SETTLED,
                                    payment_hash: Some(invoice.payment_hash),
                                    description: invoice.description,
                                    fee_msat: Some(0),
                                    payment_time: Some(Utc::now()),
                                    payment_type: PaymentType::INTERNAL,
                                    ..Default::default()
                                },
                                0.0,
                            )
                            .await?;

                        retrieved_invoice.fee_msat = Some(0);
                        retrieved_invoice.payment_time = internal_payment.payment_time;
                        self.store
                            .invoice
                            .update(Some(&txn), retrieved_invoice)
                            .await?;

                        txn.commit()
                            .await
                            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

                        return Ok(internal_payment);
                    }
                }
            }

            // External  payment
            let pending_payment = self
                .insert_payment(
                    Payment {
                        user_id,
                        amount_msat: amount,
                        status: PaymentStatus::PENDING,
                        payment_hash: Some(invoice.payment_hash),
                        description: invoice.description,
                        ..Default::default()
                    },
                    self.fee_buffer,
                )
                .await?;

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
    ) -> Result<Payment, ApplicationError> {
        let amount = Self::validate_amount(amount_msat)?;

        // Check if internal payment
        if data.domain == self.domain {
            match data.ln_address.clone() {
                Some(ln_address) => {
                    let address = ln_address.split('@').next().unwrap();
                    let address_opt = self
                        .store
                        .lightning
                        .find_address_by_username(&address)
                        .await?;

                    match address_opt {
                        Some(retrieved_address) => {
                            if retrieved_address.user_id == user_id {
                                return Err(DataError::Validation(
                                    "Cannot pay to own lightning address.".to_string(),
                                )
                                .into());
                            }

                            // Internal payment
                            let curr_time = Utc::now();

                            let txn = self.store.lightning.begin().await?;
                            self.store
                                .invoice
                                .insert(
                                    Some(&txn),
                                    Invoice {
                                        user_id: retrieved_address.user_id,
                                        lightning_address: Some(retrieved_address.id),
                                        network: Network::Bitcoin.to_string(),
                                        description: comment.clone().or(
                                            DEFAULT_INTERNAL_INVOICE_DESCRIPTION.to_string().into(),
                                        ),
                                        amount_msat: Some(amount),
                                        timestamp: curr_time,
                                        status: InvoiceStatus::SETTLED,
                                        invoice_type: InvoiceType::INTERNAL,
                                        fee_msat: Some(0),
                                        payment_time: Some(curr_time),
                                        ..Default::default()
                                    },
                                )
                                .await?;

                            let internal_payment = self
                                .insert_payment(
                                    Payment {
                                        user_id,
                                        amount_msat: amount,
                                        status: PaymentStatus::SETTLED,
                                        description: comment.or(
                                            DEFAULT_INTERNAL_PAYMENT_DESCRIPTION.to_string().into(),
                                        ),
                                        fee_msat: Some(0),
                                        payment_time: Some(Utc::now()),
                                        payment_type: PaymentType::INTERNAL,
                                        lightning_address: Some(ln_address),
                                        ..Default::default()
                                    },
                                    0.0,
                                )
                                .await?;

                            txn.commit()
                                .await
                                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

                            return Ok(internal_payment);
                        }
                        None => {
                            return Err(DataError::NotFound(
                                "Lightning address not found.".to_string(),
                            )
                            .into());
                        }
                    }
                }
                None => {
                    return Err(DataError::Validation(
                        "Invalid LNURL, Lighting address must be defined for internal payments."
                            .to_string(),
                    )
                    .into());
                }
            }
        }

        let pending_payment = self
            .insert_payment(
                Payment {
                    user_id: user_id.clone(),
                    amount_msat: amount,
                    status: PaymentStatus::PENDING,
                    lightning_address: data.ln_address.clone(),
                    description: comment.clone(),
                    ..Default::default()
                },
                self.fee_buffer,
            )
            .await?;

        let result = self
            .lightning_client
            .lnurl_pay(data, amount, comment, pending_payment.id)
            .await;

        self.handle_processed_payment(pending_payment, result).await
    }

    async fn insert_payment(
        &self,
        payment: Payment,
        fee_buffer: f64,
    ) -> Result<Payment, ApplicationError> {
        let txn = self.store.lightning.begin().await?;

        let balance = self
            .store
            .user
            .get_balance(Some(&txn), &payment.user_id)
            .await?
            .available_msat as f64;

        let required_balance = payment.amount_msat as f64 * (1.0 + fee_buffer);
        if balance < required_balance {
            return Err(DataError::InsufficientFunds(required_balance).into());
        }

        let pending_payment = self.store.payment.insert(Some(&txn), payment).await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        return Ok(pending_payment);
    }

    async fn handle_processed_payment(
        &self,
        mut pending_payment: Payment,
        result: Result<Payment, LightningError>,
    ) -> Result<Payment, ApplicationError> {
        match result {
            Ok(mut payment) => {
                payment.id = pending_payment.id;
                payment.status = match payment.error {
                    // There is a case where the payment fails but still has a payment_hash and a payment is returned instead of an error
                    Some(_) => PaymentStatus::FAILED,
                    None => PaymentStatus::SETTLED,
                };
                let payment: Payment = self.store.payment.update(payment).await?;
                Ok(payment)
            }
            Err(error) => {
                pending_payment.status = PaymentStatus::FAILED;
                pending_payment.error = Some(error.to_string());
                let payment: Payment = self.store.payment.update(pending_payment).await?;
                Ok(payment)
            }
        }
    }
}

#[async_trait]
impl PaymentsUseCases for PaymentsService {
    async fn get(&self, id: Uuid) -> Result<Payment, ApplicationError> {
        trace!(%id, "Fetching lightning payment");

        let lightning_payment = self
            .store
            .payment
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning payment not found.".to_string()))?;

        debug!(
            %id,
            "Lightning payment fetched successfully"
        );
        Ok(lightning_payment)
    }

    async fn list(&self, filter: PaymentFilter) -> Result<Vec<Payment>, ApplicationError> {
        trace!(?filter, "Listing lightning payments");

        let lightning_payments = self.store.payment.find_many(filter.clone()).await?;

        debug!(?filter, "Lightning payments listed successfully");
        Ok(lightning_payments)
    }

    async fn pay(&self, req: SendPaymentRequest) -> Result<Payment, ApplicationError> {
        debug!(?req, "Sending payment");

        let input_type = parse(&req.input)
            .await
            .map_err(|e| DataError::Validation(e.to_string()))?;

        let payment = match input_type {
            InputType::Bolt11 { invoice } => {
                self.send_bolt11(req.user_id.unwrap(), invoice, req.amount_msat)
                    .await
            }
            InputType::LnUrlPay { data } => {
                self.send_lnurl_pay(req.user_id.unwrap(), data, req.amount_msat, req.comment)
                    .await
            }
            InputType::LnUrlError { data } => Err(DataError::Validation(data.reason).into()),
            _ => Err(DataError::Validation("Unsupported payment input".to_string()).into()),
        }?;

        info!(
            id = payment.id.to_string(),
            "Payment processed successfully"
        );

        Ok(payment)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting lightning payment");

        let n_deleted = self
            .store
            .payment
            .delete_many(PaymentFilter {
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

    async fn delete_many(&self, filter: PaymentFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting lightning payments");

        let n_deleted = self.store.payment.delete_many(filter.clone()).await?;

        info!(
            ?filter,
            n_deleted, "Lightning payments deleted successfully"
        );
        Ok(n_deleted)
    }
}
