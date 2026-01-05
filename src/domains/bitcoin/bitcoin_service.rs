use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    application::{
        entities::{AppStore, Ledger},
        errors::{ApplicationError, DataError},
    },
    domains::invoice::{Invoice, InvoiceStatus},
    infra::lightning::LnClient,
};

use super::{BitcoinAddress, BitcoinOutput, BitcoinUseCases};

const DEFAULT_DEPOSIT_DESCRIPTION: &str = "Bitcoin onchain deposit";

pub struct BitcoinService {
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
}

impl BitcoinService {
    pub fn new(store: AppStore, ln_client: Arc<dyn LnClient>) -> Self {
        Self { store, ln_client }
    }

    fn invoice_status_for_output(output: &BitcoinOutput) -> InvoiceStatus {
        if output.block_height.is_some() {
            InvoiceStatus::Settled
        } else {
            InvoiceStatus::Pending
        }
    }
}

#[async_trait]
impl BitcoinUseCases for BitcoinService {
    async fn get_deposit_address(&self, wallet_id: Uuid) -> Result<BitcoinAddress, ApplicationError> {
        debug!(%wallet_id, "Fetching current bitcoin deposit address");

        if let Some(address) = self.store.btc_address.find_by_wallet_unused(wallet_id).await? {
            return Ok(address);
        }

        let address = self
            .ln_client
            .get_new_bitcoin_address()
            .await
            .map_err(|err| DataError::Validation(err.to_string()))?;

        if !self.ln_client.validate_bitcoin_address(&address).await.unwrap_or(false) {
            return Err(DataError::Validation("Invalid bitcoin address returned by node".to_string()).into());
        }

        let btc_address = self
            .store
            .btc_address
            .insert(BitcoinAddress {
                id: Uuid::new_v4(),
                wallet_id,
                address,
                used: false,
                derivation_index: None,
                created_at: Utc::now(),
                updated_at: None,
            })
            .await?;

        info!(%wallet_id, address = %btc_address.address, "New bitcoin deposit address issued");

        Ok(btc_address)
    }

    async fn sync_outputs(&self) -> Result<Vec<BitcoinOutput>, ApplicationError> {
        debug!("Syncing bitcoin outputs from lightning node");

        let outputs = self.ln_client.list_bitcoin_outputs().await?;
        let mut persisted = Vec::new();

        for mut output in outputs {
            if output.amount_sat <= 0 {
                continue;
            }

            // Ensure outpoint is always present for database consistency
            if output.outpoint.is_empty() {
                output.outpoint = format!("{}:{}", output.txid, output.output_index);
            }

            let stored_output = self.store.btc_output.upsert(output.clone()).await?;

            if let Some(address) = output.address {
                if let Some(btc_address) = self.store.btc_address.find_by_address(&address).await? {
                    if !btc_address.used {
                        self.store.btc_address.mark_used(btc_address.id).await?;
                    }

                    let existing_invoice = self.store.invoice.find_by_btc_output_id(stored_output.id).await?;

                    let status = Self::invoice_status_for_output(&stored_output);

                    if let Some(mut invoice) = existing_invoice {
                        let updated_status = Self::invoice_status_for_output(&stored_output);
                        if invoice.status != updated_status {
                            invoice.status = updated_status;
                            invoice.payment_time = stored_output.timestamp;
                            invoice.amount_received_msat = Some((stored_output.amount_sat as u64).saturating_mul(1000));
                            invoice.btc_output_id = Some(stored_output.id);
                            invoice.bitcoin_output = Some(stored_output.clone());
                            self.store.invoice.update(invoice).await?;
                        }
                    } else {
                        let timestamp = stored_output.timestamp.unwrap_or_else(Utc::now);
                        let amount_msat = (stored_output.amount_sat as u64).saturating_mul(1000);

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
                            currency: stored_output.currency.clone(),
                            fee_msat: stored_output
                                .fee_sat
                                .and_then(|fee| fee.max(0).checked_mul(1000))
                                .map(|fee| fee as u64),
                            payment_time: stored_output.timestamp,
                            ln_invoice: None,
                            btc_output_id: Some(stored_output.id),
                            bitcoin_output: Some(stored_output.clone()),
                            ..Default::default()
                        };

                        self.store.invoice.insert(invoice).await?;
                    }
                } else {
                    warn!(
                        address,
                        "Ignoring bitcoin output that does not match any known wallet address"
                    );
                }
            }

            persisted.push(stored_output);
        }

        info!(count = persisted.len(), "Bitcoin outputs synced");
        Ok(persisted)
    }
}
