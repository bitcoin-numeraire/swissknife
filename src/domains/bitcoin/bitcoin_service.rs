use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, BitcoinWallet, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::{
        invoice::{InvoiceFilter, InvoiceStatus},
        payment::{PaymentFilter, PaymentStatus},
    },
};

use super::{BitcoinAddress, BitcoinAddressType, BitcoinEventsUseCases, BitcoinUseCases};

pub struct BitcoinService {
    store: AppStore,
    wallet: Arc<dyn BitcoinWallet>,
    bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
    address_type: BitcoinAddressType,
}

impl BitcoinService {
    pub fn new(
        store: AppStore,
        wallet: Arc<dyn BitcoinWallet>,
        bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
        address_type: BitcoinAddressType,
    ) -> Self {
        Self {
            store,
            wallet,
            bitcoin_events,
            address_type,
        }
    }
}

#[async_trait]
impl BitcoinUseCases for BitcoinService {
    async fn get_deposit_address(
        &self,
        wallet_id: Uuid,
        address_type: Option<BitcoinAddressType>,
    ) -> Result<BitcoinAddress, ApplicationError> {
        let address_type = address_type.unwrap_or(self.address_type);

        trace!(%wallet_id, %address_type, "Fetching current bitcoin deposit address");

        if let Some(address) = self
            .store
            .btc_address
            .find_by_wallet_unused(wallet_id, address_type)
            .await?
        {
            return Ok(address);
        }

        let address = self.wallet.new_address(address_type).await?;

        let btc_address = self
            .store
            .btc_address
            .insert(BitcoinAddress {
                id: Uuid::new_v4(),
                wallet_id,
                address,
                address_type,
                used: false,
                created_at: Utc::now(),
                updated_at: None,
            })
            .await?;

        info!(%wallet_id, address = %btc_address.address, "New bitcoin deposit address issued");

        Ok(btc_address)
    }

    async fn sync_pending_transactions(&self) -> Result<(), ApplicationError> {
        trace!("Syncing pending onchain invoices and payments");

        let mut n_invoices = 0;
        let mut n_payments = 0;
        let network = self.wallet.network();

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
                return Err(DataError::Inconsistency("Invoice references nonexistent btc_output".to_string()).into());
            };

            let transaction = self.wallet.get_transaction(&output.txid).await?;

            let Some(matching_output) = transaction
                .outputs
                .iter()
                .find(|tx_output| tx_output.output_index == output.output_index)
            else {
                continue;
            };

            self.bitcoin_events
                .onchain_deposit(transaction.output_event(matching_output, network))
                .await?;

            n_invoices += 1;
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
                return Err(DataError::Inconsistency("Payment without transaction hash".to_string()).into());
            };

            let transaction = self.wallet.get_transaction(&txid).await?;
            let candidate_output = transaction
                .outputs
                .iter()
                .filter(|output| output.amount_sat > 0)
                .filter(|output| !output.is_ours)
                .find(|output| match (&payment.destination_address, &output.address) {
                    (Some(destination), Some(address)) => destination == address,
                    (Some(_), None) => false,
                    (None, _) => true,
                })
                .or_else(|| {
                    // If the address is not found, we can assume this output is the withdrawal.
                    // This is a CLN limitation, as it does not provide the address of the output.
                    transaction
                        .outputs
                        .iter()
                        .filter(|output| output.amount_sat > 0)
                        .find(|output| !output.is_ours)
                });

            if let Some(output) = candidate_output {
                self.bitcoin_events
                    .onchain_withdrawal(transaction.output_event(output, network))
                    .await?;

                n_payments += 1;
            }
        }

        debug!(%n_invoices, %n_payments, "Synced pending onchain invoices and payments");
        Ok(())
    }
}
