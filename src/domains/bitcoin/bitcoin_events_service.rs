use async_trait::async_trait;
use tracing::{info, trace, warn};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::{
        invoice::{Invoice, InvoiceStatus},
        payment::PaymentStatus,
    },
};

use super::{BitcoinEventsUseCases, BitcoinOutput, BitcoinOutputEvent, BtcOutputStatus};

const DEFAULT_DEPOSIT_DESCRIPTION: &str = "Bitcoin onchain deposit";

pub struct BitcoinEventsService {
    store: AppStore,
}

impl BitcoinEventsService {
    pub fn new(store: AppStore) -> Self {
        Self { store }
    }

    fn output_status(block_height: u32) -> BtcOutputStatus {
        if block_height > 0 {
            BtcOutputStatus::Confirmed
        } else {
            BtcOutputStatus::Unconfirmed
        }
    }
}

#[async_trait]
impl BitcoinEventsUseCases for BitcoinEventsService {
    async fn onchain_deposit(&self, event: BitcoinOutputEvent) -> Result<(), ApplicationError> {
        let outpoint = format!("{}:{}", event.txid, event.output_index);
        trace!(%outpoint, "Processing onchain deposit event");

        let Some(address) = event.address.clone() else {
            warn!(txid = %event.txid, "Output address not found, skipping");
            return Ok(());
        };

        let status = Self::output_status(event.block_height);
        let output = BitcoinOutput {
            id: Uuid::new_v4(),
            outpoint: outpoint.clone(),
            txid: event.txid.clone(),
            output_index: event.output_index,
            address: address.clone(),
            amount_sat: event.amount_sat,
            status,
            timestamp: event.timestamp,
            block_height: Some(event.block_height),
            network: event.network,
            ..Default::default()
        };

        let Some(btc_address) = self.store.btc_address.find_by_address(&address).await? else {
            trace!(%address, outpoint = outpoint.clone(),
                "Ignoring bitcoin output not matching any known wallet address"
            );
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
                invoice.payment_time = Some(stored_output.timestamp);
            }
            invoice.amount_received_msat = Some(stored_output.amount_sat.saturating_mul(1000));
            invoice.btc_output_id = Some(stored_output.id);
            invoice.bitcoin_output = Some(stored_output.clone());

            self.store.invoice.update(invoice.clone()).await?;

            info!(invoice_id = %invoice.id, outpoint = outpoint.clone(), address = %btc_address.address,
                "Existing onchain deposit processed");
        } else {
            let amount_msat = stored_output.amount_sat.saturating_mul(1000);
            let payment_time = if is_confirmed {
                Some(stored_output.timestamp)
            } else {
                None
            };

            let invoice = Invoice {
                id: Uuid::new_v4(),
                wallet_id: btc_address.wallet_id,
                description: Some(DEFAULT_DEPOSIT_DESCRIPTION.to_string()),
                amount_msat: Some(amount_msat),
                amount_received_msat: Some(amount_msat),
                timestamp: stored_output.timestamp,
                ledger: Ledger::Onchain,
                currency: stored_output.network.into(),
                payment_time,
                btc_output_id: Some(stored_output.id),
                bitcoin_output: Some(stored_output.clone()),
                ..Default::default()
            };

            self.store.invoice.insert(invoice.clone()).await?;

            info!(invoice_id = %invoice.id, outpoint = %outpoint.clone(), address = %btc_address.address,
                "New onchain deposit processed");
        }

        Ok(())
    }

    async fn onchain_withdrawal(&self, event: BitcoinOutputEvent) -> Result<(), ApplicationError> {
        let outpoint = format!("{}:{}", event.txid, event.output_index);
        trace!(outpoint = outpoint.clone(), "Processing onchain withdrawal event");

        let Some(payment) = self.store.payment.find_by_payment_hash(&event.txid).await? else {
            trace!(
                outpoint = outpoint.clone(),
                "Ignoring bitcoin output not matching any known payment"
            );
            return Ok(());
        };

        let is_confirmed = event.block_height > 0;

        let status: PaymentStatus = if is_confirmed {
            PaymentStatus::Settled
        } else {
            payment.status.clone()
        };

        let mut updated_payment = payment;
        updated_payment.status = status;

        if is_confirmed && updated_payment.payment_time.is_none() {
            updated_payment.payment_time = Some(event.timestamp);
        }

        if updated_payment.fee_msat.is_none() {
            updated_payment.fee_msat = event.fee_sat.map(|fee| fee.saturating_mul(1000));
        }

        let status = Self::output_status(event.block_height);

        let Some(destination_address) = updated_payment.destination_address.clone() else {
            return Err(DataError::Inconsistency("Destination address not found.".into()).into());
        };

        let btc_output = BitcoinOutput {
            id: Uuid::new_v4(),
            outpoint: outpoint.clone(),
            txid: event.txid.clone(),
            output_index: event.output_index,
            address: destination_address,
            amount_sat: event.amount_sat,
            status,
            timestamp: event.timestamp,
            block_height: Some(event.block_height),
            network: event.network,
            ..Default::default()
        };

        let stored_output = self.store.btc_output.upsert(btc_output).await?;
        updated_payment.bitcoin_output = Some(stored_output.clone());

        if updated_payment.btc_output_id.is_none() {
            updated_payment.btc_output_id = Some(stored_output.id);
        }

        self.store.payment.update(updated_payment).await?;

        info!(%outpoint, "Onchain withdrawal processed");
        Ok(())
    }
}
