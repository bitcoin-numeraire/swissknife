use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, info, trace};

use crate::{
    application::{
        entities::{AppStore, Currency, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::{
        bitcoin::{BtcOutput, BtcOutputStatus},
        event::{
            EventUseCases, LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent, OnchainDepositEvent,
            OnchainWithdrawalEvent,
        },
        invoice::{Invoice, InvoiceStatus},
        payment::PaymentStatus,
    },
};

const DEFAULT_DEPOSIT_DESCRIPTION: &str = "Bitcoin On-chain deposit";

#[derive(Clone)]
pub struct EventService {
    store: AppStore,
}

impl EventService {
    pub fn new(store: AppStore) -> Self {
        EventService { store }
    }

    fn output_status(block_height: Option<u32>) -> BtcOutputStatus {
        match block_height {
            Some(height) if height > 0 => BtcOutputStatus::Confirmed,
            _ => BtcOutputStatus::Unconfirmed,
        }
    }
}

#[async_trait]
impl EventUseCases for EventService {
    async fn invoice_paid(&self, event: LnInvoicePaidEvent) -> Result<(), ApplicationError> {
        debug!(?event, "Processing incoming Lightning payment...");

        let invoice_option = self.store.invoice.find_by_payment_hash(&event.payment_hash).await?;

        if let Some(mut invoice) = invoice_option {
            invoice.status = InvoiceStatus::Settled;
            invoice.fee_msat = Some(event.fee_msat);
            invoice.payment_time = Some(event.payment_time);
            invoice.amount_received_msat = Some(event.amount_received_msat);

            invoice = self.store.invoice.update(invoice).await?;

            info!(id = %invoice.id, "Incoming Lightning payment processed successfully");
            return Ok(());
        }

        return Err(DataError::NotFound("Lightning invoice not found.".into()).into());
    }

    async fn outgoing_payment(&self, event: LnPaySuccessEvent) -> Result<(), ApplicationError> {
        debug!(?event, "Processing outgoing Lightning payment...");

        let payment_option = self.store.payment.find_by_payment_hash(&event.payment_hash).await?;

        if let Some(mut payment_retrieved) = payment_option {
            if payment_retrieved.status == PaymentStatus::Settled {
                debug!(id = %payment_retrieved.id,"Lightning payment already settled");
                return Ok(());
            }

            payment_retrieved.status = PaymentStatus::Settled;
            payment_retrieved.payment_time = Some(event.payment_time);
            payment_retrieved.amount_msat = event.amount_msat;
            payment_retrieved.fee_msat = Some(event.fees_msat);
            let lightning = payment_retrieved.lightning.get_or_insert_with(Default::default);
            lightning.payment_preimage = Some(event.payment_preimage);

            let payment = self.store.payment_uow.settle(payment_retrieved).await?;

            info!(id = %payment.id, payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully");

            return Ok(());
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }

    async fn failed_payment(&self, event: LnPayFailureEvent) -> Result<(), ApplicationError> {
        debug!(?event, "Processing failed outgoing Lightning payment");

        let payment_option = self.store.payment.find_by_payment_hash(&event.payment_hash).await?;

        if let Some(mut payment_retrieved) = payment_option {
            if payment_retrieved.status == PaymentStatus::Failed {
                debug!(id = %payment_retrieved.id, "Lightning payment already failed");
                return Ok(());
            }

            payment_retrieved.status = PaymentStatus::Failed;
            payment_retrieved.error = Some(event.reason);

            let payment = self.store.payment_uow.fail(payment_retrieved).await?;

            info!(id = %payment.id,payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully");

            return Ok(());
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }

    async fn onchain_deposit(&self, event: OnchainDepositEvent, currency: Currency) -> Result<bool, ApplicationError> {
        let outpoint = format!("{}:{}", event.txid, event.output_index);
        trace!(%outpoint, "Processing onchain deposit event");

        let address = event.address;
        let status = Self::output_status(event.block_height);
        let output = BtcOutput {
            outpoint: outpoint.clone(),
            txid: event.txid.clone(),
            output_index: event.output_index,
            address: address.clone(),
            amount_sat: event.amount_sat,
            status,
            block_height: event.block_height,
            ..Default::default()
        };

        let Some(btc_address) = self.store.btc_address.find_by_address(&address).await? else {
            trace!(%address, outpoint = outpoint.clone(),
                "Ignoring bitcoin output not matching any known wallet address");
            return Ok(false);
        };

        let stored_output = self.store.btc_output.upsert(output.clone()).await?;

        if !btc_address.used {
            self.store.btc_address.mark_used(btc_address.id).await?;
        }

        let existing_invoice = self.store.invoice.find_by_btc_output_id(stored_output.id).await?;
        let status: InvoiceStatus = stored_output.status.into();

        let is_confirmed = stored_output.status == BtcOutputStatus::Confirmed;

        if let Some(mut invoice) = existing_invoice {
            invoice.status = status;
            if is_confirmed {
                invoice.payment_time = Some(Utc::now());
                invoice.amount_received_msat = Some(stored_output.amount_sat.saturating_mul(1000));
            }
            invoice.btc_output_id = Some(stored_output.id);
            invoice.bitcoin_output = Some(stored_output.clone());

            self.store.invoice.update(invoice.clone()).await?;

            info!(invoice_id = %invoice.id, outpoint = outpoint.clone(), address = %btc_address.address,
                "Existing onchain deposit processed");
        } else {
            let amount_msat = stored_output.amount_sat.saturating_mul(1000);
            let payment_time = if is_confirmed { Some(Utc::now()) } else { None };
            let amount_received_msat = if is_confirmed { Some(amount_msat) } else { None };

            let invoice = Invoice {
                wallet_id: btc_address.wallet_id,
                description: Some(DEFAULT_DEPOSIT_DESCRIPTION.to_string()),
                amount_msat: Some(amount_msat),
                amount_received_msat,
                timestamp: Utc::now(),
                ledger: Ledger::Onchain,
                currency,
                payment_time,
                btc_output_id: Some(stored_output.id),
                bitcoin_output: Some(stored_output.clone()),
                ..Default::default()
            };

            let stored_invoice = self.store.invoice.insert(invoice.clone()).await?;

            info!(invoice_id = %stored_invoice.id, outpoint = %outpoint.clone(), address = %btc_address.address,
                "New onchain deposit processed");
        }

        Ok(true)
    }

    async fn onchain_withdrawal(&self, event: OnchainWithdrawalEvent) -> Result<bool, ApplicationError> {
        trace!(txid = %event.txid, block_height = event.block_height, "Processing onchain withdrawal event");

        let block_height = match event.block_height {
            Some(height) if height > 0 => height,
            _ => {
                trace!(txid = %event.txid, "Bitcoin transaction not yet confirmed, ignoring for now");
                return Ok(false);
            }
        };

        let Some(mut payment) = self.store.payment.find_by_payment_hash(&event.txid).await? else {
            trace!(txid = %event.txid, "Ignoring bitcoin output not matching any known payment");
            return Ok(false);
        };

        payment.status = PaymentStatus::Settled;
        payment.payment_time = Some(Utc::now());

        let bitcoin = payment.bitcoin.get_or_insert_with(Default::default);
        bitcoin.block_height = Some(block_height);

        let stored_payment = self.store.payment_uow.settle(payment).await?;

        info!(payment_id = %stored_payment.id, txid = %event.txid, "Onchain withdrawal processed");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::entities::MockAppStoreBuilder,
        domains::{bitcoin::BtcAddress, payment::Payment},
    };

    use super::*;

    fn service(store: MockAppStoreBuilder) -> EventService {
        EventService::new(store.build())
    }

    fn btc_address(used: bool) -> BtcAddress {
        BtcAddress {
            id: Uuid::new_v4(),
            wallet_id: Uuid::new_v4(),
            address: "bc1qknown".to_string(),
            used,
            address_type: crate::domains::bitcoin::BtcAddressType::P2wpkh,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    mod invoice_paid {
        use super::*;

        mod when_invoice_exists {
            use super::*;

            #[tokio::test]
            async fn settles_the_invoice() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_find_by_payment_hash().times(1).returning(|_| {
                    Ok(Some(Invoice {
                        status: InvoiceStatus::Pending,
                        ..Default::default()
                    }))
                });
                store
                    .invoice
                    .expect_update()
                    .withf(|invoice| {
                        invoice.status == InvoiceStatus::Settled && invoice.amount_received_msat == Some(2_000)
                    })
                    .times(1)
                    .returning(Ok);

                let event = LnInvoicePaidEvent {
                    payment_hash: "ph".to_string(),
                    amount_received_msat: 2_000,
                    fee_msat: 1,
                    payment_time: Utc::now(),
                };

                assert!(service(store).invoice_paid(event).await.is_ok());
            }
        }

        mod when_invoice_is_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .invoice
                    .expect_find_by_payment_hash()
                    .times(1)
                    .returning(|_| Ok(None));

                let event = LnInvoicePaidEvent {
                    payment_hash: "ph".to_string(),
                    amount_received_msat: 2_000,
                    fee_msat: 1,
                    payment_time: Utc::now(),
                };

                let err = service(store).invoice_paid(event).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod outgoing_payment {
        use super::*;

        fn event() -> LnPaySuccessEvent {
            LnPaySuccessEvent {
                amount_msat: 1_000,
                fees_msat: 1,
                payment_hash: "ph".to_string(),
                payment_preimage: "preimage".to_string(),
                payment_time: Utc::now(),
            }
        }

        mod when_payment_is_pending {
            use super::*;

            #[tokio::test]
            async fn settles_it_with_preimage() {
                let mut store = MockAppStoreBuilder::new();
                store.payment.expect_find_by_payment_hash().times(1).returning(|_| {
                    Ok(Some(Payment {
                        status: PaymentStatus::Pending,
                        ..Default::default()
                    }))
                });
                store
                    .payment_uow
                    .expect_settle()
                    .withf(|payment| {
                        payment.status == PaymentStatus::Settled
                            && payment
                                .lightning
                                .as_ref()
                                .and_then(|lightning| lightning.payment_preimage.as_deref())
                                == Some("preimage")
                    })
                    .times(1)
                    .returning(Ok);

                assert!(service(store).outgoing_payment(event()).await.is_ok());
            }
        }

        mod when_payment_is_already_settled {
            use super::*;

            #[tokio::test]
            async fn is_idempotent_and_does_not_update() {
                let mut store = MockAppStoreBuilder::new();
                store.payment.expect_find_by_payment_hash().times(1).returning(|_| {
                    Ok(Some(Payment {
                        status: PaymentStatus::Settled,
                        ..Default::default()
                    }))
                });
                // update is intentionally not expected.

                assert!(service(store).outgoing_payment(event()).await.is_ok());
            }
        }

        mod when_payment_is_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .payment
                    .expect_find_by_payment_hash()
                    .times(1)
                    .returning(|_| Ok(None));

                let err = service(store).outgoing_payment(event()).await.unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod failed_payment {
        use super::*;

        fn event() -> LnPayFailureEvent {
            LnPayFailureEvent {
                reason: "no route".to_string(),
                payment_hash: "ph".to_string(),
            }
        }

        mod when_payment_is_pending {
            use super::*;

            #[tokio::test]
            async fn marks_it_failed_with_reason() {
                let mut store = MockAppStoreBuilder::new();
                store.payment.expect_find_by_payment_hash().times(1).returning(|_| {
                    Ok(Some(Payment {
                        status: PaymentStatus::Pending,
                        ..Default::default()
                    }))
                });
                store
                    .payment_uow
                    .expect_fail()
                    .withf(|payment| {
                        payment.status == PaymentStatus::Failed && payment.error.as_deref() == Some("no route")
                    })
                    .times(1)
                    .returning(Ok);

                assert!(service(store).failed_payment(event()).await.is_ok());
            }
        }

        mod when_payment_is_already_failed {
            use super::*;

            #[tokio::test]
            async fn is_idempotent_and_does_not_update() {
                let mut store = MockAppStoreBuilder::new();
                store.payment.expect_find_by_payment_hash().times(1).returning(|_| {
                    Ok(Some(Payment {
                        status: PaymentStatus::Failed,
                        ..Default::default()
                    }))
                });

                assert!(service(store).failed_payment(event()).await.is_ok());
            }
        }
    }

    mod onchain_deposit {
        use super::*;

        mod when_address_is_unknown {
            use super::*;

            #[tokio::test]
            async fn ignores_the_output() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_address()
                    .times(1)
                    .returning(|_| Ok(None));

                let processed = service(store)
                    .onchain_deposit(OnchainDepositEvent::default(), Currency::Regtest)
                    .await
                    .unwrap();

                assert!(!processed);
            }
        }

        mod when_address_is_known_and_confirmed {
            use super::*;

            #[tokio::test]
            async fn creates_a_settled_deposit_invoice() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .btc_address
                    .expect_find_by_address()
                    .times(1)
                    .returning(|_| Ok(Some(btc_address(false))));
                store.btc_output.expect_upsert().times(1).returning(|output| {
                    Ok(BtcOutput {
                        id: Uuid::new_v4(),
                        status: BtcOutputStatus::Confirmed,
                        amount_sat: 1_000,
                        ..output
                    })
                });
                // Address was unused, so it must be flagged as used.
                store.btc_address.expect_mark_used().times(1).returning(|_| Ok(()));
                store
                    .invoice
                    .expect_find_by_btc_output_id()
                    .times(1)
                    .returning(|_| Ok(None));
                store
                    .invoice
                    .expect_insert()
                    .withf(|invoice| {
                        invoice.ledger == Ledger::Onchain
                            && invoice.amount_received_msat == Some(1_000_000)
                            && invoice.payment_time.is_some()
                    })
                    .times(1)
                    .returning(Ok);

                let event = OnchainDepositEvent {
                    txid: "txid".to_string(),
                    output_index: 0,
                    address: "bc1qknown".to_string(),
                    amount_sat: 1_000,
                    block_height: Some(800_000),
                };

                let processed = service(store).onchain_deposit(event, Currency::Bitcoin).await.unwrap();

                assert!(processed);
            }
        }
    }

    mod onchain_withdrawal {
        use super::*;

        mod when_transaction_is_unconfirmed {
            use super::*;

            #[tokio::test]
            async fn ignores_it() {
                // No store calls expected for an unconfirmed transaction.
                let event = OnchainWithdrawalEvent {
                    txid: "txid".to_string(),
                    block_height: None,
                };

                let processed = service(MockAppStoreBuilder::new())
                    .onchain_withdrawal(event)
                    .await
                    .unwrap();

                assert!(!processed);
            }
        }

        mod when_payment_is_unknown {
            use super::*;

            #[tokio::test]
            async fn ignores_it() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .payment
                    .expect_find_by_payment_hash()
                    .times(1)
                    .returning(|_| Ok(None));

                let event = OnchainWithdrawalEvent {
                    txid: "txid".to_string(),
                    block_height: Some(800_000),
                };

                let processed = service(store).onchain_withdrawal(event).await.unwrap();

                assert!(!processed);
            }
        }

        mod when_payment_matches {
            use super::*;

            #[tokio::test]
            async fn settles_the_payment_with_block_height() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .payment
                    .expect_find_by_payment_hash()
                    .times(1)
                    .returning(|_| Ok(Some(Payment::default())));
                store
                    .payment_uow
                    .expect_settle()
                    .withf(|payment| {
                        payment.status == PaymentStatus::Settled
                            && payment.bitcoin.as_ref().and_then(|bitcoin| bitcoin.block_height) == Some(800_000)
                    })
                    .times(1)
                    .returning(Ok);

                let event = OnchainWithdrawalEvent {
                    txid: "txid".to_string(),
                    block_height: Some(800_000),
                };

                let processed = service(store).onchain_withdrawal(event).await.unwrap();

                assert!(processed);
            }
        }
    }
}
