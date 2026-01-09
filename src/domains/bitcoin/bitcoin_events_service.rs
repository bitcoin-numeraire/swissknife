use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, BitcoinWallet, Ledger},
        errors::ApplicationError,
    },
    domains::{
        invoice::{Invoice, InvoiceFilter, InvoiceStatus},
        payment::{PaymentFilter, PaymentStatus},
    },
};

use super::{BitcoinEventsUseCases, BitcoinOutput, BitcoinOutputStatus, BitcoinTransaction, BitcoinTransactionOutput};

const DEFAULT_DEPOSIT_DESCRIPTION: &str = "Bitcoin onchain deposit";

pub struct BitcoinEventsService {
    store: AppStore,
    wallet: Arc<dyn BitcoinWallet>,
}

impl BitcoinEventsService {
    pub fn new(store: AppStore, wallet: Arc<dyn BitcoinWallet>) -> Self {
        Self { store, wallet }
    }

    fn output_status(confirmations: Option<i64>, block_height: Option<i64>) -> BitcoinOutputStatus {
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
                invoice.amount_received_msat = Some((stored_output.amount_sat.max(0) as u64).saturating_mul(1000));
                invoice.btc_output_id = Some(stored_output.id);
                invoice.bitcoin_output = Some(stored_output.clone());
                self.store.invoice.update(invoice).await?;
            }
        } else {
            let timestamp = stored_output.timestamp.unwrap_or_else(Utc::now);
            let amount_msat = (stored_output.amount_sat.max(0) as u64).saturating_mul(1000);

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
        transaction: &BitcoinTransaction,
        outputs: &[BitcoinTransactionOutput],
    ) -> Result<(), ApplicationError> {
        let Some(payment) = self.store.payment.find_by_payment_hash(&transaction.txid).await? else {
            return Ok(());
        };

        let is_confirmed = match transaction.confirmations {
            Some(confirmations) => confirmations > 0,
            None => transaction.block_height.unwrap_or_default() > 0,
        };

        let status = if is_confirmed {
            PaymentStatus::Settled
        } else {
            payment.status.clone()
        };

        let mut updated_payment = payment;
        updated_payment.status = status;

        if updated_payment.payment_time.is_none() {
            updated_payment.payment_time = transaction.timestamp;
        }

        if updated_payment.fee_msat.is_none() {
            updated_payment.fee_msat = transaction.fee_sat.map(|fee| (fee.max(0) as u64).saturating_mul(1000));
        }

        if updated_payment.btc_output_id.is_none() {
            let candidate_output = outputs
                .iter()
                .filter(|output| !output.is_ours)
                .filter(|output| output.amount_sat > 0)
                .find(|output| match (&updated_payment.destination_address, &output.address) {
                    (Some(destination), Some(address)) => destination == address,
                    (Some(_), None) => false,
                    (None, _) => true,
                })
                .or_else(|| outputs.iter().find(|output| !output.is_ours && output.amount_sat > 0));

            if let Some(output) = candidate_output {
                let network = self.wallet.network().await?;
                let status = Self::output_status(transaction.confirmations, transaction.block_height);

                let btc_output = BitcoinOutput {
                    id: Uuid::new_v4(),
                    outpoint: format!("{}:{}", transaction.txid, output.output_index),
                    txid: transaction.txid.clone(),
                    output_index: output.output_index,
                    address: output.address.clone(),
                    amount_sat: output.amount_sat,
                    status,
                    timestamp: transaction.timestamp,
                    block_height: transaction.block_height,
                    network,
                    created_at: Utc::now(),
                    updated_at: None,
                };

                let stored_output = self.store.btc_output.upsert(btc_output).await?;
                updated_payment.btc_output_id = Some(stored_output.id);
                updated_payment.bitcoin_output = Some(stored_output);
            }
        }

        self.store.payment.update(updated_payment).await?;

        Ok(())
    }
}

#[async_trait]
impl BitcoinEventsUseCases for BitcoinEventsService {
    async fn onchain_transaction(&self, transaction: BitcoinTransaction) -> Result<(), ApplicationError> {
        debug!(txid = %transaction.txid, "Processing onchain transaction event");

        let network = self.wallet.network().await?;
        let status = Self::output_status(transaction.confirmations, transaction.block_height);

        let needs_address_lookup = transaction.outputs.iter().any(|output| output.address.is_none());
        let utxos = if needs_address_lookup {
            Some(self.wallet.list_outputs().await?)
        } else {
            None
        };

        let mut resolved_outputs = Vec::new();
        for output in transaction.outputs.iter() {
            let outpoint = format!("{}:{}", transaction.txid, output.output_index);
            let mut resolved_address = output.address.clone();
            let mut is_ours = output.is_ours;

            if resolved_address.is_none() {
                if let Some(utxos) = &utxos {
                    if let Some(utxo) = utxos.iter().find(|candidate| candidate.outpoint == outpoint) {
                        resolved_address = utxo.address.clone();
                        is_ours = true;
                    }
                }
            }

            let candidate = BitcoinOutput {
                id: Uuid::new_v4(),
                outpoint,
                txid: transaction.txid.clone(),
                output_index: output.output_index,
                address: resolved_address,
                amount_sat: output.amount_sat,
                status,
                timestamp: transaction.timestamp,
                block_height: transaction.block_height,
                network,
                created_at: Utc::now(),
                updated_at: None,
            };

            if is_ours {
                if let Some(stored) = self.apply_output(candidate).await? {
                    resolved_outputs.push(BitcoinTransactionOutput {
                        output_index: stored.output_index,
                        address: stored.address.clone(),
                        amount_sat: stored.amount_sat,
                        is_ours: true,
                    });
                }
            } else {
                resolved_outputs.push(BitcoinTransactionOutput {
                    output_index: output.output_index,
                    address: output.address.clone(),
                    amount_sat: output.amount_sat,
                    is_ours: false,
                });
            }
        }

        self.update_payment(&transaction, &resolved_outputs).await?;

        info!(txid = %transaction.txid, "Onchain transaction processed");
        Ok(())
    }

    async fn sync_pending_transactions(&self) -> Result<(), ApplicationError> {
        debug!("Syncing pending onchain invoices and payments");

        let invoices = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                ledger: Some(Ledger::Onchain),
                status: Some(InvoiceStatus::Pending),
                ..Default::default()
            })
            .await?;

        for invoice in invoices {
            let Some(btc_output_id) = invoice.btc_output_id else {
                continue;
            };

            let Some(output) = self.store.btc_output.find(btc_output_id).await? else {
                continue;
            };

            let transaction = self.wallet.get_transaction(&output.txid).await?;
            self.onchain_transaction(transaction).await?;
        }

        let payments = self
            .store
            .payment
            .find_many(PaymentFilter {
                ledger: Some(Ledger::Onchain),
                status: Some(PaymentStatus::Pending),
                ..Default::default()
            })
            .await?;

        for payment in payments {
            let Some(txid) = payment.payment_hash else {
                continue;
            };

            let transaction = self.wallet.get_transaction(&txid).await?;
            self.onchain_transaction(transaction).await?;
        }

        Ok(())
    }
}
