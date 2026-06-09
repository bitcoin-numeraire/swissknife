use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use lightning_invoice::Bolt11Invoice;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, Currency, Ledger},
        errors::{ApplicationError, DataError, LightningError},
    },
    domains::{
        bitcoin::BitcoinWallet,
        event::{EventUseCases, LnPayFailureEvent, LnPaySuccessEvent},
        invoice::{Invoice, InvoiceStatus},
        lnurl::{process_success_action, validate_lnurl_pay, LnUrlPayRequestData, LnUrlPaySuccessAction},
    },
    infra::lightning::LnClient,
};

use super::{
    payment_input::{parse_payment_input, BitcoinAddressData, ParsedBolt11Invoice, PaymentInput},
    BtcPayment, InternalPayment, LnPayment, Payment, PaymentFilter, PaymentStatus, PaymentsUseCases,
};

const DEFAULT_INTERNAL_INVOICE_DESCRIPTION: &str = "Numeraire Invoice";
const DEFAULT_INTERNAL_PAYMENT_DESCRIPTION: &str = "Payment to Numeraire";

pub struct PaymentService {
    domain: String,
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
    bitcoin_wallet: Arc<dyn BitcoinWallet>,
    fee_buffer: f64,
    events: Arc<dyn EventUseCases>,
}

impl PaymentService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
        domain: String,
        fee_buffer: f64,
        events: Arc<dyn EventUseCases>,
    ) -> Self {
        PaymentService {
            store,
            ln_client,
            bitcoin_wallet,
            domain,
            fee_buffer,
            events,
        }
    }
}

impl PaymentService {
    pub(crate) fn validate_amount(amount_msat: Option<u64>) -> Result<u64, ApplicationError> {
        let amount = amount_msat.unwrap_or_default();
        if amount == 0 {
            return Err(DataError::Validation("Amount must be greater than zero".to_string()).into());
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

        let (username, _) = input.split_once('@').expect("should not fail or malformed LN address");

        let address_opt = self.store.ln_address.find_by_username(username).await?;
        match address_opt {
            Some(retrieved_address) => {
                if !retrieved_address.active {
                    return Err(DataError::NotFound("Recipient not found.".to_string()).into());
                }

                if retrieved_address.wallet_id == wallet_id {
                    return Err(DataError::Validation("Cannot pay to yourself.".to_string()).into());
                }

                let curr_time = Utc::now();

                self.store
                    .invoice
                    .insert(Invoice {
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
                    .store
                    .payment_uow
                    .insert_payment(
                        Payment {
                            wallet_id,
                            amount_msat: amount,
                            status: PaymentStatus::Settled,
                            description: comment.or(DEFAULT_INTERNAL_PAYMENT_DESCRIPTION.to_string().into()),
                            fee_msat: Some(0),
                            payment_time: Some(curr_time),
                            ledger: Ledger::Internal,
                            currency: Currency::Bitcoin,
                            internal: Some(InternalPayment {
                                ln_address: Some(input),
                                btc_address: None,
                                payment_hash: None,
                            }),
                            ..Default::default()
                        },
                        0.0,
                    )
                    .await?;

                Ok(internal_payment)
            }
            None => Err(DataError::NotFound("Recipient not found.".to_string()).into()),
        }
    }

    async fn send_bitcoin(
        &self,
        data: BitcoinAddressData,
        amount_sat: Option<u64>,
        comment: Option<String>,
        wallet_id: Uuid,
    ) -> Result<Payment, ApplicationError> {
        let specified_amount = data.amount_sat.or(amount_sat);
        if specified_amount == Some(0) {
            return Err(DataError::Validation("Amount must be greater than zero.".to_string()).into());
        }

        if let Some(amount) = specified_amount {
            let amount_msat = amount * 1000;
            let description: Option<String> = comment.or(data.message);

            if let Some(recipient_address) = self.store.btc_address.find_by_address(&data.address).await? {
                if recipient_address.wallet_id == wallet_id {
                    return Err(DataError::Validation("Cannot pay to your own bitcoin address.".to_string()).into());
                }

                let timestamp = Utc::now();

                self.store
                    .invoice
                    .insert(Invoice {
                        wallet_id: recipient_address.wallet_id,
                        ln_address_id: None,
                        description: description.clone(),
                        amount_msat: Some(amount_msat),
                        amount_received_msat: Some(amount_msat),
                        timestamp,
                        status: InvoiceStatus::Settled,
                        ledger: Ledger::Internal,
                        currency: data.network.into(),
                        fee_msat: Some(0),
                        payment_time: Some(timestamp),
                        btc_output_id: None,
                        ..Default::default()
                    })
                    .await?;

                let internal_payment = self
                    .store
                    .payment_uow
                    .insert_payment(
                        Payment {
                            wallet_id,
                            amount_msat,
                            status: PaymentStatus::Settled,
                            ledger: Ledger::Internal,
                            currency: data.network.into(),
                            description: description.clone(),
                            internal: Some(InternalPayment {
                                ln_address: None,
                                btc_address: Some(data.address),
                                payment_hash: None,
                            }),
                            fee_msat: Some(0),
                            payment_time: Some(timestamp),
                            ..Default::default()
                        },
                        0.0,
                    )
                    .await?;

                return Ok(internal_payment);
            }

            let prepared_tx = self
                .bitcoin_wallet
                .prepare_transaction(data.address.clone(), amount, None)
                .await?;

            let pending_payment = match self
                .store
                .payment_uow
                .insert_payment(
                    Payment {
                        wallet_id,
                        amount_msat,
                        fee_msat: Some(prepared_tx.fee_sat.saturating_mul(1000)),
                        status: PaymentStatus::Pending,
                        ledger: Ledger::Onchain,
                        currency: data.network.into(),
                        description,
                        bitcoin: Some(BtcPayment {
                            address: data.address,
                            txid: prepared_tx.txid.clone(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    0.0,
                )
                .await
            {
                Ok(payment) => payment,
                Err(error) => {
                    if let Err(err) = self.bitcoin_wallet.release_prepared_transaction(&prepared_tx).await {
                        warn!(txid = prepared_tx.txid.clone(), %err,
                            "Failed while inserting. Please release the tx manually or wait for lease expiration.");
                    }
                    return Err(error);
                }
            };

            match self.bitcoin_wallet.sign_send_transaction(&prepared_tx).await {
                Ok(resolved_txid) => {
                    // If sign_send returned a resolved txid, update the payment
                    // record so withdrawal events can be matched by this identifier.
                    if let Some(txid) = resolved_txid {
                        let mut updated_payment = pending_payment.clone();
                        let bitcoin = updated_payment.bitcoin.get_or_insert_with(Default::default);
                        bitcoin.txid = txid;
                        self.store.payment.update(updated_payment).await?;
                    }
                }
                Err(error) => {
                    if let Err(err) = self.bitcoin_wallet.release_prepared_transaction(&prepared_tx).await {
                        warn!(txid = prepared_tx.txid, %err,
                            "Failed while signing and sending. Please release the tx manually or wait for lease expiration.");
                    }

                    let mut failed_payment = pending_payment.clone();
                    failed_payment.status = PaymentStatus::Failed;
                    failed_payment.error = Some(error.to_string());
                    self.store.payment.update(failed_payment).await?;

                    return Err(error.into());
                }
            };

            Ok(pending_payment)
        } else {
            Err(DataError::Validation("Amount must be defined for on-chain transactions.".to_string()).into())
        }
    }

    async fn send_bolt11(
        &self,
        invoice: ParsedBolt11Invoice,
        amount_msat: Option<u64>,
        comment: Option<String>,
        wallet_id: Uuid,
    ) -> Result<Payment, ApplicationError> {
        let specified_amount = invoice.amount_msat.or(amount_msat);
        if specified_amount == Some(0) {
            return Err(DataError::Validation("Amount must be greater than zero.".to_string()).into());
        }

        if let Some(amount) = specified_amount {
            // Check if internal payment
            let invoice_opt = self.store.invoice.find_by_payment_hash(&invoice.payment_hash).await?;
            if let Some(mut retrieved_invoice) = invoice_opt {
                if retrieved_invoice.wallet_id == wallet_id {
                    return Err(DataError::Validation("Cannot pay for own invoice.".to_string()).into());
                }

                match retrieved_invoice.status {
                    InvoiceStatus::Settled => {
                        return Err(DataError::Validation("Invoice has already been paid.".to_string()).into());
                    }
                    InvoiceStatus::Expired => {
                        return Err(DataError::Validation("Invoice is expired.".to_string()).into());
                    }
                    InvoiceStatus::Pending => {
                        // Internal payment
                        debug!(%wallet_id, %amount, ledger="Internal", "Sending bolt11 payment");

                        let payment_hash = invoice.payment_hash.clone();
                        let internal_payment = self
                            .store
                            .payment_uow
                            .insert_payment(
                                Payment {
                                    wallet_id,
                                    amount_msat: amount,
                                    status: PaymentStatus::Settled,
                                    description: invoice.description,
                                    fee_msat: Some(0),
                                    payment_time: Some(Utc::now()),
                                    ledger: Ledger::Internal,
                                    currency: invoice.currency.clone(),
                                    internal: Some(InternalPayment {
                                        ln_address: None,
                                        btc_address: None,
                                        payment_hash: Some(payment_hash.clone()),
                                    }),
                                    ..Default::default()
                                },
                                0.0,
                            )
                            .await?;

                        let invoice_id = retrieved_invoice.id;
                        retrieved_invoice.fee_msat = Some(0);
                        retrieved_invoice.payment_time = internal_payment.payment_time;
                        retrieved_invoice.amount_received_msat = Some(amount);
                        retrieved_invoice.ledger = Ledger::Internal;
                        self.store.invoice.update(retrieved_invoice).await?;

                        if let Err(err) = self
                            .ln_client
                            .cancel_invoice(payment_hash.clone(), invoice_id.to_string())
                            .await
                        {
                            warn!(
                                %wallet_id,
                                payment_hash = %payment_hash,
                                %err,
                                "Failed to cancel node invoice after internal payment"
                            );
                        }

                        return Ok(internal_payment);
                    }
                }
            }

            // External  payment
            debug!(%wallet_id, %amount, ledger="Lightning", "Sending bolt11 payment");

            let pending_payment = self
                .store
                .payment_uow
                .insert_payment(
                    Payment {
                        wallet_id,
                        amount_msat: amount,
                        status: PaymentStatus::Pending,
                        ledger: Ledger::Lightning,
                        currency: invoice.currency.clone(),
                        description: comment,
                        lightning: Some(LnPayment {
                            payment_hash: invoice.payment_hash,
                            ..Default::default()
                        }),
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
                    pending_payment.id.to_string(),
                )
                .await;

            self.handle_processed_payment(pending_payment, result, None).await
        } else {
            Err(DataError::Validation("Amount must be defined for zero-amount invoices.".to_string()).into())
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
            .store
            .payment_uow
            .insert_payment(
                Payment {
                    wallet_id,
                    amount_msat: amount,
                    status: PaymentStatus::Pending,
                    description: comment.clone(),
                    ledger: Ledger::Lightning,
                    currency: Currency::Bitcoin,
                    lightning: Some(LnPayment {
                        ln_address: data.ln_address.clone(),
                        payment_hash: Bolt11Invoice::from_str(&cb.pr)
                            .expect("should not fail or malformed callback")
                            .payment_hash()
                            .to_string(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                self.fee_buffer,
            )
            .await?;

        let result = self.ln_client.pay(cb.pr, None, pending_payment.id.to_string()).await;

        self.handle_processed_payment(pending_payment, result, cb.success_action)
            .await
    }

    async fn handle_processed_payment(
        &self,
        mut pending_payment: Payment,
        result: Result<Payment, LightningError>,
        success_action: Option<LnUrlPaySuccessAction>,
    ) -> Result<Payment, ApplicationError> {
        match result {
            Ok(mut settled_payment) => {
                settled_payment.id = pending_payment.id;
                settled_payment.status = PaymentStatus::Settled;

                let success_action = match (
                    success_action,
                    settled_payment
                        .lightning
                        .as_ref()
                        .and_then(|lightning| lightning.payment_preimage.as_deref()),
                ) {
                    (Some(sa), Some(preimage)) => process_success_action(sa, preimage),
                    _ => None,
                };

                if let Some(success_action) = success_action {
                    let lightning = settled_payment.lightning.get_or_insert_with(Default::default);
                    lightning.success_action = Some(success_action);
                }

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
            self.send_internal(input, amount_msat, comment, wallet_id).await
        } else {
            let input_type = parse_payment_input(&input).await.map_err(DataError::Validation)?;

            match input_type {
                PaymentInput::BitcoinAddress(address) => {
                    let amount_sat = amount_msat.map(|amount| amount / 1000);
                    self.send_bitcoin(address, amount_sat, comment, wallet_id).await
                }
                PaymentInput::Bolt11(invoice) => self.send_bolt11(invoice, amount_msat, comment, wallet_id).await,
                PaymentInput::LnUrlPay(data) => self.send_lnurl_pay(data, amount_msat, comment, wallet_id).await,
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

    async fn sync(&self) -> Result<u32, ApplicationError> {
        trace!("Synchronizing pending payments...");

        let pending_payments = self
            .store
            .payment
            .find_many(PaymentFilter {
                status: Some(PaymentStatus::Pending),
                ledger: Some(Ledger::Lightning),
                ..Default::default()
            })
            .await?;

        let mut synced = 0;

        for payment in pending_payments {
            let Some(lightning) = payment.lightning.as_ref() else {
                return Err(DataError::Inconsistency(format!(
                    "Missing lightning metadata on lightning payment with id: {}",
                    payment.id
                ))
                .into());
            };
            let payment_hash = lightning.payment_hash.clone();

            let Some(node_payment) = self.ln_client.payment_by_hash(payment_hash.clone()).await? else {
                continue;
            };

            match node_payment.status {
                PaymentStatus::Settled => {
                    let payment_time = node_payment.payment_time.unwrap_or_else(Utc::now);
                    let payment_preimage = node_payment
                        .lightning
                        .as_ref()
                        .and_then(|lightning| lightning.payment_preimage.clone())
                        .unwrap_or_default();

                    let event = LnPaySuccessEvent {
                        amount_msat: node_payment.amount_msat,
                        fees_msat: node_payment.fee_msat.unwrap_or_default(),
                        payment_hash: payment_hash.clone(),
                        payment_preimage,
                        payment_time,
                    };

                    self.events.outgoing_payment(event).await?;
                    synced += 1;
                }
                PaymentStatus::Failed => {
                    let reason = node_payment
                        .error
                        .clone()
                        .unwrap_or_else(|| "Payment failed".to_string());
                    self.events
                        .failed_payment(LnPayFailureEvent { payment_hash, reason })
                        .await?;

                    synced += 1;
                }
                PaymentStatus::Pending => {
                    debug!(payment_id = %payment.id, "Payment still pending; skipping sync");
                    continue;
                }
            }
        }

        debug!(synced, "Pending payments synchronized successfully");
        Ok(synced)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{entities::MockAppStoreBuilder, errors::BitcoinError},
        domains::{
            bitcoin::{BtcAddress, BtcAddressType, BtcNetwork, BtcPreparedTransaction, MockBitcoinWallet},
            event::MockEventUseCases,
            ln_address::LnAddress,
        },
        infra::lightning::MockLnClient,
    };

    use super::*;

    const DOMAIN: &str = "numeraire.tech";
    const FEE_BUFFER: f64 = 0.02;

    fn service(
        store: MockAppStoreBuilder,
        ln_client: MockLnClient,
        bitcoin_wallet: MockBitcoinWallet,
        events: MockEventUseCases,
    ) -> PaymentService {
        PaymentService::new(
            store.build(),
            Arc::new(ln_client),
            Arc::new(bitcoin_wallet),
            DOMAIN.to_string(),
            FEE_BUFFER,
            Arc::new(events),
        )
    }

    fn ln_address(wallet_id: Uuid, active: bool) -> LnAddress {
        LnAddress {
            id: Uuid::new_v4(),
            wallet_id,
            username: "bob".to_string(),
            active,
            allows_nostr: false,
            nostr_pubkey: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    fn btc_address(wallet_id: Uuid) -> BtcAddress {
        BtcAddress {
            id: Uuid::new_v4(),
            wallet_id,
            address: "bcrt1qrecipient".to_string(),
            used: false,
            address_type: BtcAddressType::P2wpkh,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    fn bitcoin_data(amount_sat: Option<u64>) -> BitcoinAddressData {
        BitcoinAddressData {
            address: "bcrt1qrecipient".to_string(),
            amount_sat,
            message: None,
            network: BtcNetwork::Regtest,
        }
    }

    fn bolt11(amount_msat: Option<u64>) -> ParsedBolt11Invoice {
        ParsedBolt11Invoice {
            bolt11: "lnbc1example".to_string(),
            amount_msat,
            payment_hash: "ph".to_string(),
            description: None,
            currency: Currency::Bitcoin,
        }
    }

    fn prepared_tx() -> BtcPreparedTransaction {
        BtcPreparedTransaction {
            txid: "txid".to_string(),
            fee_sat: 10,
            psbt: "psbt".to_string(),
            locked_utxos: vec![],
        }
    }

    mod validate_amount {
        use super::*;

        fn assert_validation_error(amount: Option<u64>) {
            let err = PaymentService::validate_amount(amount).unwrap_err();
            assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            assert!(err.to_string().contains("Amount must be greater than zero"));
        }

        mod with_positive_amount {
            use super::*;

            #[test]
            fn returns_amount() {
                assert_eq!(PaymentService::validate_amount(Some(1)).unwrap(), 1);
                assert_eq!(PaymentService::validate_amount(Some(42_000)).unwrap(), 42_000);
            }
        }

        mod without_amount {
            use super::*;

            #[test]
            fn rejects_missing_amount() {
                assert_validation_error(None);
            }
        }

        mod with_zero_amount {
            use super::*;

            #[test]
            fn rejects_zero() {
                assert_validation_error(Some(0));
            }
        }
    }

    mod is_internal_payment {
        use super::*;

        #[test]
        fn matches_the_configured_domain() {
            let service = service(
                MockAppStoreBuilder::new(),
                MockLnClient::new(),
                MockBitcoinWallet::new(),
                MockEventUseCases::new(),
            );

            assert!(service.is_internal_payment("bob@numeraire.tech"));
            assert!(!service.is_internal_payment("bob@example.com"));
            assert!(!service.is_internal_payment("not-an-address"));
        }
    }

    mod send_internal {
        use super::*;

        mod when_recipient_is_active_and_distinct {
            use super::*;

            #[tokio::test]
            async fn writes_counterpart_invoice_and_settled_payment() {
                let sender = Uuid::new_v4();
                let recipient = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .withf(|username| username == "bob")
                    .times(1)
                    .returning(move |_| Ok(Some(ln_address(recipient, true))));
                store
                    .invoice
                    .expect_insert()
                    .withf(move |invoice| invoice.wallet_id == recipient && invoice.status == InvoiceStatus::Settled)
                    .times(1)
                    .returning(Ok);
                store
                    .payment_uow
                    .expect_insert_payment()
                    .withf(|payment, fee_buffer| payment.ledger == Ledger::Internal && *fee_buffer < f64::EPSILON)
                    .times(1)
                    .returning(|payment, _| Ok(payment));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let payment = service
                    .send_internal("bob@numeraire.tech".to_string(), Some(1_000), None, sender)
                    .await
                    .unwrap();

                assert_eq!(payment.status, PaymentStatus::Settled);
                assert_eq!(payment.ledger, Ledger::Internal);
            }
        }

        mod with_zero_amount {
            use super::*;

            #[tokio::test]
            async fn rejects_before_lookup() {
                let service = service(
                    MockAppStoreBuilder::new(),
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_internal("bob@numeraire.tech".to_string(), Some(0), None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod when_recipient_is_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(None));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_internal("bob@numeraire.tech".to_string(), Some(1_000), None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod when_recipient_is_inactive {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(Some(ln_address(Uuid::new_v4(), false))));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_internal("bob@numeraire.tech".to_string(), Some(1_000), None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod when_paying_yourself {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let wallet_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(move |_| Ok(Some(ln_address(wallet_id, true))));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_internal("bob@numeraire.tech".to_string(), Some(1_000), None, wallet_id)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
                assert!(err.to_string().contains("Cannot pay to yourself"));
            }
        }
    }

    mod pay {
        use super::*;

        mod with_an_internal_ln_address {
            use super::*;

            #[tokio::test]
            async fn routes_to_the_internal_flow() {
                let recipient = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(move |_| Ok(Some(ln_address(recipient, true))));
                store.invoice.expect_insert().times(1).returning(Ok);
                store
                    .payment_uow
                    .expect_insert_payment()
                    .times(1)
                    .returning(|payment, _| Ok(payment));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let payment = service
                    .pay("bob@numeraire.tech".to_string(), Some(1_000), None, Uuid::new_v4())
                    .await
                    .unwrap();

                assert_eq!(payment.ledger, Ledger::Internal);
            }
        }
    }

    mod send_bitcoin {
        use super::*;

        mod with_zero_amount {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let service = service(
                    MockAppStoreBuilder::new(),
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_bitcoin(bitcoin_data(Some(0)), None, None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod without_amount {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let service = service(
                    MockAppStoreBuilder::new(),
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_bitcoin(bitcoin_data(None), None, None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
                assert!(err.to_string().contains("Amount must be defined"));
            }
        }

        mod when_address_belongs_to_a_local_wallet {
            use super::*;

            #[tokio::test]
            async fn settles_as_an_internal_payment() {
                let recipient = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_address()
                    .times(1)
                    .returning(move |_| Ok(Some(btc_address(recipient))));
                store
                    .invoice
                    .expect_insert()
                    .withf(move |invoice| invoice.wallet_id == recipient && invoice.ledger == Ledger::Internal)
                    .times(1)
                    .returning(Ok);
                store
                    .payment_uow
                    .expect_insert_payment()
                    .times(1)
                    .returning(|payment, _| Ok(payment));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let payment = service
                    .send_bitcoin(bitcoin_data(Some(1_000)), None, None, Uuid::new_v4())
                    .await
                    .unwrap();

                assert_eq!(payment.status, PaymentStatus::Settled);
                assert_eq!(payment.ledger, Ledger::Internal);
            }
        }

        mod when_paying_your_own_bitcoin_address {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let wallet_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_address()
                    .times(1)
                    .returning(move |_| Ok(Some(btc_address(wallet_id))));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_bitcoin(bitcoin_data(Some(1_000)), None, None, wallet_id)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod when_broadcasting_externally {
            use super::*;

            #[tokio::test]
            async fn reserves_then_signs_and_returns_pending() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_address()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .payment_uow
                    .expect_insert_payment()
                    .withf(|payment, _| payment.status == PaymentStatus::Pending && payment.ledger == Ledger::Onchain)
                    .times(1)
                    .returning(|payment, _| Ok(payment));

                let mut wallet = MockBitcoinWallet::new();
                wallet
                    .expect_prepare_transaction()
                    .times(1)
                    .returning(|_, _, _| Ok(prepared_tx()));
                // No resolved txid, so the payment row is not updated afterwards.
                wallet.expect_sign_send_transaction().times(1).returning(|_| Ok(None));

                let service = service(store, MockLnClient::new(), wallet, MockEventUseCases::new());

                let payment = service
                    .send_bitcoin(bitcoin_data(Some(1_000)), None, None, Uuid::new_v4())
                    .await
                    .unwrap();

                assert_eq!(payment.status, PaymentStatus::Pending);
            }
        }

        mod when_reservation_fails {
            use super::*;

            #[tokio::test]
            async fn releases_the_prepared_transaction_and_propagates() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_address()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .payment_uow
                    .expect_insert_payment()
                    .times(1)
                    .returning(|_, _| Err(DataError::InsufficientFunds(1_000.0).into()));

                let mut wallet = MockBitcoinWallet::new();
                wallet
                    .expect_prepare_transaction()
                    .times(1)
                    .returning(|_, _, _| Ok(prepared_tx()));
                // The reservation failed before broadcast, so the lease must be released.
                wallet
                    .expect_release_prepared_transaction()
                    .times(1)
                    .returning(|_| Ok(()));

                let service = service(store, MockLnClient::new(), wallet, MockEventUseCases::new());

                let err = service
                    .send_bitcoin(bitcoin_data(Some(1_000)), None, None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::InsufficientFunds(_))));
            }
        }

        mod when_signing_fails {
            use super::*;

            #[tokio::test]
            async fn releases_lease_and_marks_payment_failed() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_address()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .payment_uow
                    .expect_insert_payment()
                    .times(1)
                    .returning(|payment, _| Ok(payment));
                store
                    .payment
                    .expect_update()
                    .withf(|payment| payment.status == PaymentStatus::Failed)
                    .times(1)
                    .returning(Ok);

                let mut wallet = MockBitcoinWallet::new();
                wallet
                    .expect_prepare_transaction()
                    .times(1)
                    .returning(|_, _, _| Ok(prepared_tx()));
                wallet
                    .expect_sign_send_transaction()
                    .times(1)
                    .returning(|_| Err(BitcoinError::FinalizeTransaction("rejected".to_string())));
                wallet
                    .expect_release_prepared_transaction()
                    .times(1)
                    .returning(|_| Ok(()));

                let service = service(store, MockLnClient::new(), wallet, MockEventUseCases::new());

                let err = service
                    .send_bitcoin(bitcoin_data(Some(1_000)), None, None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Bitcoin(_)));
            }
        }
    }

    mod send_bolt11 {
        use super::*;

        mod when_invoice_is_internal_and_pending {
            use super::*;

            #[tokio::test]
            async fn settles_internally_and_cancels_node_invoice() {
                let sender = Uuid::new_v4();
                let recipient = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .invoice
                    .expect_find_by_payment_hash()
                    .times(1)
                    .returning(move |_| {
                        Ok(Some(Invoice {
                            wallet_id: recipient,
                            status: InvoiceStatus::Pending,
                            ..Default::default()
                        }))
                    });
                store
                    .payment_uow
                    .expect_insert_payment()
                    .withf(|payment, _| payment.ledger == Ledger::Internal)
                    .times(1)
                    .returning(|payment, _| Ok(payment));
                store
                    .invoice
                    .expect_update()
                    .withf(move |invoice| invoice.ledger == Ledger::Internal)
                    .times(1)
                    .returning(Ok);

                let mut ln_client = MockLnClient::new();
                ln_client.expect_cancel_invoice().times(1).returning(|_, _| Ok(()));

                let service = service(store, ln_client, MockBitcoinWallet::new(), MockEventUseCases::new());

                let payment = service
                    .send_bolt11(bolt11(Some(1_000)), None, None, sender)
                    .await
                    .unwrap();

                assert_eq!(payment.status, PaymentStatus::Settled);
            }
        }

        mod when_paying_your_own_invoice {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let wallet_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .invoice
                    .expect_find_by_payment_hash()
                    .times(1)
                    .returning(move |_| {
                        Ok(Some(Invoice {
                            wallet_id,
                            status: InvoiceStatus::Pending,
                            ..Default::default()
                        }))
                    });

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_bolt11(bolt11(Some(1_000)), None, None, wallet_id)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
            }
        }

        mod when_internal_invoice_is_already_settled {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_find_by_payment_hash().times(1).returning(|_| {
                    Ok(Some(Invoice {
                        wallet_id: Uuid::new_v4(),
                        status: InvoiceStatus::Settled,
                        ..Default::default()
                    }))
                });

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_bolt11(bolt11(Some(1_000)), None, None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
                assert!(err.to_string().contains("already been paid"));
            }
        }

        mod when_internal_invoice_is_expired {
            use super::*;

            #[tokio::test]
            async fn returns_validation_error() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_find_by_payment_hash().times(1).returning(|_| {
                    Ok(Some(Invoice {
                        wallet_id: Uuid::new_v4(),
                        status: InvoiceStatus::Expired,
                        ..Default::default()
                    }))
                });

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service
                    .send_bolt11(bolt11(Some(1_000)), None, None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
                assert!(err.to_string().contains("expired"));
            }
        }

        mod when_invoice_is_external {
            use super::*;

            #[tokio::test]
            async fn reserves_pays_and_settles() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .invoice
                    .expect_find_by_payment_hash()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .payment_uow
                    .expect_insert_payment()
                    .withf(|payment, fee_buffer| payment.ledger == Ledger::Lightning && *fee_buffer > 0.0)
                    .times(1)
                    .returning(|payment, _| Ok(payment));
                store
                    .payment
                    .expect_update()
                    .withf(|payment| payment.status == PaymentStatus::Settled)
                    .times(1)
                    .returning(Ok);

                let mut ln_client = MockLnClient::new();
                ln_client.expect_pay().times(1).returning(|_, _, _| {
                    Ok(Payment {
                        status: PaymentStatus::Settled,
                        lightning: Some(LnPayment {
                            payment_preimage: Some("preimage".to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                });

                let service = service(store, ln_client, MockBitcoinWallet::new(), MockEventUseCases::new());

                let payment = service
                    .send_bolt11(bolt11(Some(1_000)), None, None, Uuid::new_v4())
                    .await
                    .unwrap();

                assert_eq!(payment.status, PaymentStatus::Settled);
            }
        }

        mod when_external_payment_fails {
            use super::*;

            #[tokio::test]
            async fn marks_payment_failed_and_propagates() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .invoice
                    .expect_find_by_payment_hash()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .payment_uow
                    .expect_insert_payment()
                    .times(1)
                    .returning(|payment, _| Ok(payment));
                store
                    .payment
                    .expect_update()
                    .withf(|payment| payment.status == PaymentStatus::Failed)
                    .times(1)
                    .returning(Ok);

                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_pay()
                    .times(1)
                    .returning(|_, _, _| Err(LightningError::Pay("no route".to_string())));

                let service = service(store, ln_client, MockBitcoinWallet::new(), MockEventUseCases::new());

                let err = service
                    .send_bolt11(bolt11(Some(1_000)), None, None, Uuid::new_v4())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Lightning(_)));
            }
        }
    }

    mod handle_processed_payment {
        use super::*;

        mod when_payment_succeeds {
            use super::*;

            #[tokio::test]
            async fn persists_a_settled_payment() {
                let pending_id = Uuid::new_v4();

                let mut store = MockAppStoreBuilder::new();
                store
                    .payment
                    .expect_update()
                    .withf(move |payment| payment.id == pending_id && payment.status == PaymentStatus::Settled)
                    .times(1)
                    .returning(Ok);

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let pending = Payment {
                    id: pending_id,
                    status: PaymentStatus::Pending,
                    ..Default::default()
                };
                let settled = Payment {
                    status: PaymentStatus::Settled,
                    ..Default::default()
                };

                let payment = service
                    .handle_processed_payment(pending, Ok(settled), None)
                    .await
                    .unwrap();

                assert_eq!(payment.status, PaymentStatus::Settled);
            }
        }

        mod when_payment_fails {
            use super::*;

            #[tokio::test]
            async fn persists_failure_and_returns_error() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .payment
                    .expect_update()
                    .withf(|payment| payment.status == PaymentStatus::Failed && payment.error.is_some())
                    .times(1)
                    .returning(Ok);

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let pending = Payment {
                    status: PaymentStatus::Pending,
                    ..Default::default()
                };

                let err = service
                    .handle_processed_payment(pending, Err(LightningError::Pay("boom".to_string())), None)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Lightning(_)));
            }
        }
    }

    mod get {
        use super::*;

        mod when_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.payment.expect_find().times(1).returning(|_| Ok(None));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service.get(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod delete {
        use super::*;

        mod when_nothing_is_removed {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store.payment.expect_delete_many().times(1).returning(|_| Ok(0));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service.delete(Uuid::new_v4()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod sync {
        use super::*;

        fn pending_lightning_payment() -> Payment {
            Payment {
                id: Uuid::new_v4(),
                status: PaymentStatus::Pending,
                ledger: Ledger::Lightning,
                lightning: Some(LnPayment {
                    payment_hash: "ph".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }
        }

        mod when_node_reports_settled {
            use super::*;

            #[tokio::test]
            async fn dispatches_outgoing_payment_event() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .payment
                    .expect_find_many()
                    .times(1)
                    .returning(|_| Ok(vec![pending_lightning_payment()]));

                let mut ln_client = MockLnClient::new();
                ln_client.expect_payment_by_hash().times(1).returning(|_| {
                    Ok(Some(Payment {
                        status: PaymentStatus::Settled,
                        amount_msat: 1_000,
                        fee_msat: Some(1),
                        payment_time: Some(Utc::now()),
                        lightning: Some(LnPayment {
                            payment_preimage: Some("preimage".to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }))
                });

                let mut events = MockEventUseCases::new();
                events
                    .expect_outgoing_payment()
                    .withf(|event| event.payment_hash == "ph")
                    .times(1)
                    .returning(|_| Ok(()));

                let service = service(store, ln_client, MockBitcoinWallet::new(), events);

                assert_eq!(service.sync().await.unwrap(), 1);
            }
        }

        mod when_lightning_metadata_is_missing {
            use super::*;

            #[tokio::test]
            async fn returns_inconsistency_error() {
                let mut store = MockAppStoreBuilder::new();
                store.payment.expect_find_many().times(1).returning(|_| {
                    Ok(vec![Payment {
                        status: PaymentStatus::Pending,
                        ledger: Ledger::Lightning,
                        lightning: None,
                        ..Default::default()
                    }])
                });

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                let err = service.sync().await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::Inconsistency(_))));
            }
        }

        mod when_there_are_no_pending_payments {
            use super::*;

            #[tokio::test]
            async fn returns_zero() {
                let mut store = MockAppStoreBuilder::new();
                store.payment.expect_find_many().times(1).returning(|_| Ok(vec![]));

                let service = service(
                    store,
                    MockLnClient::new(),
                    MockBitcoinWallet::new(),
                    MockEventUseCases::new(),
                );

                assert_eq!(service.sync().await.unwrap(), 0);
            }
        }
    }
}
