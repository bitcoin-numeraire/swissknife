use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, info, trace, warn};

use crate::{
    application::{
        entities::{AppStore, Currency, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::{
        bitcoin::{BtcOutput, BtcOutputStatus},
        event::{
            BtcOutputEvent, BtcWithdrawalConfirmedEvent, EventUseCases, LnInvoicePaidEvent, LnPayFailureEvent,
            LnPaySuccessEvent,
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

            let payment = self.store.payment.update(payment_retrieved).await?;

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

            let payment = self.store.payment.update(payment_retrieved).await?;

            info!(id = %payment.id,payment_status = %payment.status,
                "Outgoing Lightning payment processed successfully");

            return Ok(());
        }

        return Err(DataError::NotFound("Lightning payment not found.".into()).into());
    }

    async fn onchain_deposit(&self, event: BtcOutputEvent, currency: Currency) -> Result<(), ApplicationError> {
        let outpoint = format!("{}:{}", event.txid, event.output_index);
        trace!(%outpoint, "Processing onchain deposit event");

        let Some(address) = event.address.clone() else {
            warn!(txid = %event.txid, "Output address not found, skipping");
            return Ok(());
        };

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
            return Ok(());
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
            }
            invoice.amount_received_msat = Some(stored_output.amount_sat.saturating_mul(1000));
            invoice.btc_output_id = Some(stored_output.id);
            invoice.bitcoin_output = Some(stored_output.clone());

            self.store.invoice.update(invoice.clone()).await?;

            info!(invoice_id = %invoice.id, outpoint = outpoint.clone(), address = %btc_address.address,
                "Existing onchain deposit processed");
        } else {
            let amount_msat = stored_output.amount_sat.saturating_mul(1000);
            let payment_time = if is_confirmed { Some(Utc::now()) } else { None };

            let invoice = Invoice {
                wallet_id: btc_address.wallet_id,
                description: Some(DEFAULT_DEPOSIT_DESCRIPTION.to_string()),
                amount_msat: Some(amount_msat),
                amount_received_msat: Some(amount_msat),
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

        Ok(())
    }

    async fn onchain_withdrawal(&self, event: BtcOutputEvent) -> Result<(), ApplicationError> {
        let outpoint = format!("{}:{}", event.txid, event.output_index);
        trace!(outpoint = outpoint.clone(), "Processing onchain withdrawal event");

        let Some(payment) = self.store.payment.find_by_payment_hash(&event.txid).await? else {
            trace!(
                outpoint = outpoint.clone(),
                "Ignoring bitcoin output not matching any known payment (probably change output)"
            );
            return Ok(());
        };

        let status = Self::output_status(event.block_height);
        let is_confirmed = status == BtcOutputStatus::Confirmed;

        let payment_status: PaymentStatus = if is_confirmed {
            PaymentStatus::Settled
        } else {
            payment.status.clone()
        };

        let mut updated_payment = payment;
        updated_payment.status = payment_status;

        if is_confirmed && updated_payment.payment_time.is_none() {
            updated_payment.payment_time = Some(Utc::now());
        }

        let status = Self::output_status(event.block_height);

        let Some(destination_address) = updated_payment
            .bitcoin
            .as_ref()
            .and_then(|b| b.destination_address.clone())
        else {
            return Err(DataError::Inconsistency("Destination address not found.".into()).into());
        };

        let btc_output = BtcOutput {
            outpoint: outpoint.clone(),
            txid: event.txid.clone(),
            output_index: event.output_index,
            address: destination_address,
            amount_sat: event.amount_sat,
            status,
            block_height: event.block_height,
            ..Default::default()
        };

        let stored_output = self.store.btc_output.upsert(btc_output).await?;
        let bitcoin = updated_payment.bitcoin.get_or_insert_with(Default::default);
        bitcoin.output = Some(stored_output.clone());

        if bitcoin.output_id.is_none() {
            bitcoin.output_id = Some(stored_output.id);
        }

        self.store.payment.update(updated_payment).await?;

        info!(%outpoint, "Onchain withdrawal processed");
        Ok(())
    }

    async fn onchain_withdrawal_confirmed(&self, event: BtcWithdrawalConfirmedEvent) -> Result<(), ApplicationError> {
        if event.block_height == 0 {
            debug!(txid = %event.txid, "Ignoring withdrawal confirmation with zero block height");
            return Ok(());
        }

        trace!(
            txid = %event.txid,
            block_height = event.block_height,
            "Processing onchain withdrawal confirmation"
        );

        let Some(mut payment) = self.store.payment.find_by_payment_hash(&event.txid).await? else {
            debug!(txid = %event.txid, "Ignoring withdrawal confirmation for unknown payment");
            return Ok(());
        };

        payment.status = PaymentStatus::Settled;

        if payment.payment_time.is_none() {
            payment.payment_time = Some(Utc::now());
        }

        if let Some(output) = payment.bitcoin.as_ref().and_then(|bitcoin| bitcoin.output.clone()) {
            let mut updated_output = output.clone();
            let block_height = Some(event.block_height);
            updated_output.block_height = block_height;
            updated_output.status = Self::output_status(block_height);

            let stored_output = self.store.btc_output.upsert(updated_output).await?;
            let bitcoin = payment.bitcoin.get_or_insert_with(Default::default);
            bitcoin.output = Some(stored_output.clone());
            if bitcoin.output_id.is_none() {
                bitcoin.output_id = Some(stored_output.id);
            }
        } else {
            warn!(txid = %event.txid, "Withdrawal output missing; updating payment only");
        }

        self.store.payment.update(payment).await?;

        info!(txid = %event.txid, "Onchain withdrawal confirmed");
        Ok(())
    }
}
