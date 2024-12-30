use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use breez_sdk_core::{parse, InputType, LNInvoice, LnUrlPayRequestData, SuccessAction};
use chrono::Utc;
use lightning_invoice::Bolt11Invoice;
use serde_bolt::bitcoin::hashes::hex::ToHex;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, Currency, Ledger},
        errors::{ApplicationError, DataError, DatabaseError, LightningError},
    },
    domains::{
        invoice::{Invoice, InvoiceStatus},
        lnurl::{process_success_action, validate_lnurl_pay},
    },
    infra::lightning::LnClient,
};

use super::{Payment, PaymentFilter, PaymentStatus, PaymentsUseCases};

const DEFAULT_INTERNAL_INVOICE_DESCRIPTION: &str = "Numeraire Invoice";
const DEFAULT_INTERNAL_PAYMENT_DESCRIPTION: &str = "Payment to Numeraire";

pub struct PaymentService {
    domain: String,
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
    fee_buffer: f64,
}

impl PaymentService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        domain: String,
        fee_buffer: f64,
    ) -> Self {
        PaymentService {
            store,
            ln_client,
            domain,
            fee_buffer,
        }
    }
}

impl PaymentService {
    pub(crate) fn validate_amount(amount_msat: Option<u64>) -> Result<u64, ApplicationError> {
        let amount = amount_msat.unwrap_or_default();
        if amount == 0 {
            return Err(
                DataError::Validation("Amount must be greater than zero".to_string()).into(),
            );
        }

        Ok(amount)
    }

    async fn send_internal(
        &self,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
        wallet_id: Uuid,
    ) -> Result<Payment, ApplicationError> {
        let amount = Self::validate_amount(amount_msat)?;
        debug!(%wallet_id, %amount, ledger="Internal", "Sending internal payment");

        let (username, _) = input
            .split_once('@')
            .expect("should not fail or malformed LN address");

        let address_opt = self.store.ln_address.find_by_username(username).await?;
        match address_opt {
            Some(retrieved_address) => {
                if retrieved_address.wallet_id == wallet_id {
                    return Err(DataError::Validation("Cannot pay to yourself.".to_string()).into());
                }

                let curr_time = Utc::now();

                let txn = self.store.begin().await?;
                self.store
                    .invoice
                    .insert(Invoice {
                        id: Uuid::new_v4(),
                        wallet_id: retrieved_address.wallet_id,
                        ln_address_id: Some(retrieved_address.id),
                        ledger: Ledger::Internal,
                        currency: Currency::Bitcoin,
                        description: comment
                            .clone()
                            .or(DEFAULT_INTERNAL_INVOICE_DESCRIPTION.to_string().into()),
                        amount_msat: Some(amount),
                        amount_received_msat: Some(amount),
                        timestamp: curr_time,
                        status: InvoiceStatus::Settled,
                        fee_msat: Some(0),
                        payment_time: Some(curr_time),
                        ..Default::default()
                    })
                    .await?;

                let internal_payment = self
                    .insert_payment(
                        Payment {
                            wallet_id,
                            amount_msat: amount,
                            status: PaymentStatus::Settled,
                            description: comment
                                .or(DEFAULT_INTERNAL_PAYMENT_DESCRIPTION.to_string().into()),
                            fee_msat: Some(0),
                            payment_time: Some(curr_time),
                            ledger: Ledger::Internal,
                            currency: Currency::Bitcoin,
                            ln_address: Some(input),
                            ..Default::default()
                        },
                        0.0,
                    )
                    .await?;

                txn.commit()
                    .await
                    .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

                Ok(internal_payment)
            }
            None => Err(DataError::NotFound("Recipient not found.".to_string()).into()),
        }
    }

    async fn send_bolt11(
        &self,
        invoice: LNInvoice,
        amount_msat: Option<u64>,
        comment: Option<String>,
        wallet_id: Uuid,
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
                if retrieved_invoice.wallet_id == wallet_id {
                    return Err(
                        DataError::Validation("Cannot pay for own invoice.".to_string()).into(),
                    );
                }

                match retrieved_invoice.status {
                    InvoiceStatus::Settled => {
                        return Err(DataError::Validation(
                            "Invoice has already been paid.".to_string(),
                        )
                        .into());
                    }
                    InvoiceStatus::Expired => {
                        return Err(DataError::Validation("Invoice is expired.".to_string()).into());
                    }
                    InvoiceStatus::Pending => {
                        // Internal payment
                        debug!(%wallet_id, %amount, ledger="Internal", "Sending bolt11 payment");

                        let internal_payment = self
                            .insert_payment(
                                Payment {
                                    wallet_id,
                                    amount_msat: amount,
                                    status: PaymentStatus::Settled,
                                    payment_hash: Some(invoice.payment_hash),
                                    description: invoice.description,
                                    fee_msat: Some(0),
                                    payment_time: Some(Utc::now()),
                                    ledger: Ledger::Internal,
                                    currency: invoice.network.into(),
                                    ..Default::default()
                                },
                                0.0,
                            )
                            .await?;

                        retrieved_invoice.fee_msat = Some(0);
                        retrieved_invoice.payment_time = internal_payment.payment_time;
                        retrieved_invoice.amount_received_msat = Some(amount);
                        retrieved_invoice.ledger = Ledger::Internal;
                        self.store.invoice.update(retrieved_invoice).await?;

                        return Ok(internal_payment);
                    }
                }
            }

            // External  payment
            debug!(%wallet_id, %amount, ledger="Lightning", "Sending bolt11 payment");

            let pending_payment = self
                .insert_payment(
                    Payment {
                        wallet_id,
                        amount_msat: amount,
                        status: PaymentStatus::Pending,
                        ledger: Ledger::Lightning,
                        currency: invoice.network.into(),
                        payment_hash: Some(invoice.payment_hash),
                        description: comment,
                        ..Default::default()
                    },
                    self.fee_buffer,
                )
                .await?;

            let result = self
                .ln_client
                .pay(
                    invoice.bolt11.clone(),
                    if invoice.amount_msat.is_some() {
                        None
                    } else {
                        Some(amount)
                    },
                )
                .await;

            self.handle_processed_payment(pending_payment, result, None)
                .await
        } else {
            Err(DataError::Validation(
                "Amount must be defined for zero-amount invoices.".to_string(),
            )
            .into())
        }
    }

    async fn send_lnurl_pay(
        &self,
        data: LnUrlPayRequestData,
        amount_msat: Option<u64>,
        comment: Option<String>,
        wallet_id: Uuid,
    ) -> Result<Payment, ApplicationError> {
        let amount = Self::validate_amount(amount_msat)?;
        debug!(%wallet_id, %amount, ledger="Lightning", "Sending LNURL payment");

        let cb = validate_lnurl_pay(amount, &comment, &data)
            .await
            .map_err(|e| DataError::Validation(e.to_string()))?;

        let pending_payment = self
            .insert_payment(
                Payment {
                    wallet_id,
                    amount_msat: amount,
                    status: PaymentStatus::Pending,
                    ln_address: data.ln_address.clone(),
                    description: comment.clone(),
                    payment_hash: Some(
                        Bolt11Invoice::from_str(&cb.pr)
                            .expect("should not fail or malformed callback")
                            .payment_hash()
                            .to_hex(),
                    ),
                    ledger: Ledger::Lightning,
                    currency: Currency::Bitcoin,
                    ..Default::default()
                },
                self.fee_buffer,
            )
            .await?;

        let result = self.ln_client.pay(cb.pr, None).await;

        self.handle_processed_payment(pending_payment, result, cb.success_action)
            .await
    }

    async fn insert_payment(
        &self,
        payment: Payment,
        fee_buffer: f64,
    ) -> Result<Payment, ApplicationError> {
        let txn = self.store.begin().await?;

        let balance = self
            .store
            .wallet
            .get_balance(Some(&txn), payment.wallet_id)
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

        Ok(pending_payment)
    }

    async fn handle_processed_payment(
        &self,
        mut pending_payment: Payment,
        result: Result<Payment, LightningError>,
        success_action: Option<SuccessAction>,
    ) -> Result<Payment, ApplicationError> {
        match result {
            Ok(mut settled_payment) => {
                settled_payment.id = pending_payment.id;
                settled_payment.status = PaymentStatus::Settled;

                let success_action = match success_action {
                    Some(sa) => process_success_action(
                        sa,
                        settled_payment.payment_preimage.as_ref().unwrap(),
                    ),
                    None => None,
                };

                settled_payment.success_action = success_action;

                let payment = self.store.payment.update(settled_payment).await?;

                Ok(payment)
            }
            Err(error) => {
                pending_payment.status = PaymentStatus::Failed;
                pending_payment.error = Some(error.to_string());
                self.store.payment.update(pending_payment).await?;

                Err(error.into())
            }
        }
    }

    fn is_internal_payment(&self, input: &str) -> bool {
        if let Some((_, input_domain)) = input.split_once('@') {
            return input_domain == self.domain;
        }
        false
    }
}

#[async_trait]
impl PaymentsUseCases for PaymentService {
    async fn get(&self, id: Uuid) -> Result<Payment, ApplicationError> {
        trace!(%id, "Fetching payment");

        let payment = self
            .store
            .payment
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Payment not found.".to_string()))?;

        debug!(%id,"Payment fetched successfully");
        Ok(payment)
    }

    async fn list(&self, filter: PaymentFilter) -> Result<Vec<Payment>, ApplicationError> {
        trace!(?filter, "Listing payments");

        let payments = self.store.payment.find_many(filter.clone()).await?;

        debug!(?filter, "Payments listed successfully");
        Ok(payments)
    }

    async fn pay(
        &self,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
        wallet_id: Uuid,
    ) -> Result<Payment, ApplicationError> {
        debug!(%input, %wallet_id, "Received pay request");

        let payment = if self.is_internal_payment(&input) {
            self.send_internal(input, amount_msat, comment, wallet_id)
                .await
        } else {
            let input_type = parse(&input)
                .await
                .map_err(|err| DataError::Validation(err.to_string()))?;

            match input_type {
                InputType::Bolt11 { invoice } => {
                    self.send_bolt11(invoice, amount_msat, comment, wallet_id)
                        .await
                }
                InputType::LnUrlPay { data } => {
                    self.send_lnurl_pay(data, amount_msat, comment, wallet_id)
                        .await
                }
                InputType::LnUrlError { data } => Err(DataError::Validation(data.reason).into()),
                _ => Err(DataError::Validation("Unsupported payment input".to_string()).into()),
            }
        }?;

        info!(id = %payment.id, "Payment processed successfully");
        Ok(payment)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting payment");

        let n_deleted = self
            .store
            .payment
            .delete_many(PaymentFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Payment not found.".to_string()).into());
        }

        info!(%id, "Payments deleted successfully");
        Ok(())
    }

    async fn delete_many(&self, filter: PaymentFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting payments");

        let n_deleted = self.store.payment.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Payments deleted successfully");
        Ok(n_deleted)
    }
}
