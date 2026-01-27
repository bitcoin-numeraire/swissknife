use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::{
        bitcoin::{BtcAddressFilter, BitcoinWallet},
        invoice::{InvoiceFilter, InvoiceStatus},
        payment::{PaymentFilter, PaymentStatus},
    },
};

use super::{BitcoinEventsUseCases, BitcoinUseCases, BtcAddress, BtcAddressType};

pub struct BitcoinService {
    store: AppStore,
    wallet: Arc<dyn BitcoinWallet>,
    bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
    address_type: BtcAddressType,
}

impl BitcoinService {
    pub fn new(
        store: AppStore,
        wallet: Arc<dyn BitcoinWallet>,
        bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
        address_type: BtcAddressType,
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
    async fn new_deposit_address(
        &self,
        wallet_id: Uuid,
        address_type: Option<BtcAddressType>,
    ) -> Result<BtcAddress, ApplicationError> {
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

        let btc_address = self.store.btc_address.insert(wallet_id, &address, address_type).await?;

        info!(%wallet_id, address = %btc_address.address, "New bitcoin deposit address issued");

        Ok(btc_address)
    }

    async fn get_address(&self, id: Uuid) -> Result<BtcAddress, ApplicationError> {
        trace!(%id, "Fetching Bitcoin address");

        let address = self
            .store
            .btc_address
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Bitcoin address not found.".to_string()))?;

        debug!(%id, "Bitcoin address fetched successfully");
        Ok(address)
    }

    async fn list_addresses(&self, filter: BtcAddressFilter) -> Result<Vec<BtcAddress>, ApplicationError> {
        trace!(?filter, "Listing Bitcoin addresses");

        let addresses = self.store.btc_address.find_many(filter.clone()).await?;

        debug!(?filter, "Bitcoin addresses listed successfully");
        Ok(addresses)
    }

    async fn delete_address(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting Bitcoin address");

        let n_deleted = self
            .store
            .btc_address
            .delete_many(BtcAddressFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Bitcoin address not found.".to_string()).into());
        }

        info!(%id, "Bitcoin address deleted successfully");
        Ok(())
    }

    async fn delete_many_addresses(&self, filter: BtcAddressFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting bitcoin addresses");

        let n_deleted = self.store.btc_address.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Bitcoin addresses deleted successfully");
        Ok(n_deleted)
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
                .find(|output| match (&payment.btc_address, &output.address) {
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

        debug!(%n_invoices, %n_payments, "Onchain invoices and payments synced successfully");
        Ok(())
    }
}
