use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, Ledger},
        errors::ApplicationError,
    },
    domains::{
        invoice::{Invoice, InvoiceStatus},
        payment::PaymentStatus,
    },
};

use super::{BitcoinEventsUseCases, BitcoinNetwork, BitcoinOutput, BitcoinOutputEvent, BitcoinOutputStatus};

const DEFAULT_DEPOSIT_DESCRIPTION: &str = "Bitcoin onchain deposit";

pub struct BitcoinEventsService {
    store: AppStore,
}

impl BitcoinEventsService {
    pub fn new(store: AppStore) -> Self {
        Self { store }
    }

    fn output_status(confirmations: Option<u32>, block_height: Option<u32>) -> BitcoinOutputStatus {
        match confirmations {
            Some(confirmations) if confirmations > 0 => BitcoinOutputStatus::Confirmed,
            _ => match block_height {
                Some(height) if height > 0 => BitcoinOutputStatus::Confirmed,
                _ => BitcoinOutputStatus::Unconfirmed,
            },
        }
    }

    async fn apply_output(&self, output: BitcoinOutput) -> Result<Option<BitcoinOutput>, ApplicationError> {
        let Some(address) = output.address.clone() else {
            trace!(outpoint = %output.outpoint, "Ignoring bitcoin output without address");
            return Ok(None);
        };

        let Some(btc_address) = self.store.btc_address.find_by_address(&address).await? else {
            trace!(address, "Ignoring bitcoin output not matching any known wallet address");
            return Ok(None);
        };

        let stored_output = self.store.btc_output.upsert(output).await?;

        if !btc_address.used {
            self.store.btc_address.mark_used(btc_address.id).await?;
        }

        let existing_invoice = self.store.invoice.find_by_btc_output_id(stored_output.id).await?;
        let status: InvoiceStatus = stored_output.status.into();

        if let Some(mut invoice) = existing_invoice {
            if invoice.status != status || invoice.payment_time != stored_output.timestamp {
                invoice.status = status;
                invoice.payment_time = stored_output.timestamp;
                invoice.amount_received_msat = Some(stored_output.amount_sat.saturating_mul(1000));
                invoice.btc_output_id = Some(stored_output.id);
                invoice.bitcoin_output = Some(stored_output.clone());
                self.store.invoice.update(invoice).await?;
            }
        } else {
            let timestamp = stored_output.timestamp.unwrap_or_else(Utc::now);
            let amount_msat = stored_output.amount_sat.saturating_mul(1000);

            let invoice = Invoice {
                id: Uuid::new_v4(),
                wallet_id: btc_address.wallet_id,
                ln_address_id: None,
                description: Some(DEFAULT_DEPOSIT_DESCRIPTION.to_string()),
                amount_msat: Some(amount_msat),
                amount_received_msat: Some(amount_msat),
                timestamp,
                status,
                ledger: Ledger::Onchain,
                currency: stored_output.network.into(),
                fee_msat: None,
                payment_time: stored_output.timestamp,
                ln_invoice: None,
                btc_output_id: Some(stored_output.id),
                bitcoin_output: Some(stored_output.clone()),
                ..Default::default()
            };

            self.store.invoice.insert(invoice).await?;
        }

        Ok(Some(stored_output))
    }

    async fn update_payment(
        &self,
        output_event: &BitcoinOutputEvent,
        network: BitcoinNetwork,
    ) -> Result<(), ApplicationError> {
        let Some(payment) = self.store.payment.find_by_payment_hash(&output_event.txid).await? else {
            return Ok(());
        };

        let is_confirmed = match output_event.confirmations {
            Some(confirmations) => confirmations > 0,
            None => output_event.block_height.unwrap_or_default() > 0,
        };

        let status = if is_confirmed {
            PaymentStatus::Settled
        } else {
            payment.status.clone()
        };

        let mut updated_payment = payment;
        updated_payment.status = status;

        if updated_payment.payment_time.is_none() {
            updated_payment.payment_time = output_event.timestamp;
        }

        if updated_payment.fee_msat.is_none() {
            updated_payment.fee_msat = output_event.fee_sat.map(|fee| fee.saturating_mul(1000));
        }

        if updated_payment.btc_output_id.is_none() {
            let status = Self::output_status(output_event.confirmations, output_event.block_height);

            let btc_output = BitcoinOutput {
                id: Uuid::new_v4(),
                outpoint: format!("{}:{}", output_event.txid, output_event.output_index),
                txid: output_event.txid.clone(),
                output_index: output_event.output_index,
                address: output_event.address.clone(),
                amount_sat: output_event.amount_sat,
                status,
                timestamp: output_event.timestamp,
                block_height: output_event.block_height,
                network,
                created_at: Utc::now(),
                updated_at: None,
            };

            let stored_output = self.store.btc_output.upsert(btc_output).await?;
            updated_payment.btc_output_id = Some(stored_output.id);
            updated_payment.bitcoin_output = Some(stored_output);
        }

        self.store.payment.update(updated_payment).await?;

        Ok(())
    }
}

#[async_trait]
impl BitcoinEventsUseCases for BitcoinEventsService {
    async fn onchain_deposit(
        &self,
        output: BitcoinOutputEvent,
        network: BitcoinNetwork,
    ) -> Result<(), ApplicationError> {
        trace!(txid = %output.txid, output_index = output.output_index, "Processing onchain deposit event");

        let Some(address) = output.address.clone() else {
            debug!(txid = %output.txid, "Output address not found, skipping");
            return Ok(());
        };

        let status = Self::output_status(output.confirmations, output.block_height);
        let candidate = BitcoinOutput {
            id: Uuid::new_v4(),
            outpoint: format!("{}:{}", output.txid, output.output_index),
            txid: output.txid.clone(),
            output_index: output.output_index,
            address: Some(address),
            amount_sat: output.amount_sat,
            status,
            timestamp: output.timestamp,
            block_height: output.block_height,
            network,
            created_at: Utc::now(),
            updated_at: None,
        };

        self.apply_output(candidate).await?;

        debug!(txid = %output.txid, output_index = output.output_index, "Onchain deposit processed");
        Ok(())
    }

    async fn onchain_withdrawal(
        &self,
        output: BitcoinOutputEvent,
        network: BitcoinNetwork,
    ) -> Result<(), ApplicationError> {
        trace!(txid = %output.txid, output_index = output.output_index, "Processing onchain withdrawal event");

        self.update_payment(&output, network).await?;

        debug!(txid = %output.txid, output_index = output.output_index, "Onchain withdrawal processed");
        Ok(())
    }
}
