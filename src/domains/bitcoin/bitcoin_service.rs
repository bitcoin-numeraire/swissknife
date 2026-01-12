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

use super::{BitcoinAddress, BitcoinAddressType, BitcoinUseCases};

const DEFAULT_DEPOSIT_DESCRIPTION: &str = "Bitcoin onchain deposit";

pub struct BitcoinService {
    store: AppStore,
    wallet: Arc<dyn BitcoinWallet>,
    address_type: BitcoinAddressType,
}

impl BitcoinService {
    pub fn new(store: AppStore, wallet: Arc<dyn BitcoinWallet>, address_type: BitcoinAddressType) -> Self {
        Self {
            store,
            wallet,
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
        debug!(%wallet_id, ?address_type, "Fetching current bitcoin deposit address");

        let address_type = address_type.unwrap_or(self.address_type);

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

            // TODO: Handle onchain deposit
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

            // TODO: Handle onchain withdrawal
        }

        debug!("Synced pending onchain invoices and payments");
        Ok(())
    }
}
