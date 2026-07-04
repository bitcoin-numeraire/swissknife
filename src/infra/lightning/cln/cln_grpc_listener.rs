use std::sync::Arc;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use tonic::transport::Channel;
use tracing::{trace, warn};

use super::{
    cln::{
        listchainmoves_chainmoves::ListchainmovesChainmovesPrimaryTag,
        listchainmoves_request::ListchainmovesIndex,
        node_client::NodeClient,
        wait_invoices::WaitInvoicesStatus,
        wait_request::{WaitIndexname, WaitSubsystem},
        wait_sendpays::WaitSendpaysStatus,
        waitinvoice_response::WaitinvoiceStatus,
        ListchainmovesRequest, WaitRequest, WaitinvoiceRequest, WaitsendpayRequest,
    },
    cln_grpc_client::{ClnClientConfig, ClnGrpcClient},
};
use crate::{
    application::{
        composition::{AppServices, Currency},
        errors::LightningError,
    },
    domains::{
        bitcoin::{BitcoinWallet, OnchainSyncCursor},
        event::{LnPayFailureEvent, LnPaySuccessEvent, OnchainWithdrawalEvent},
    },
    infra::lightning::EventsListener,
};

pub struct ClnGrpcListener {
    client: NodeClient<Channel>,
    services: Arc<AppServices>,
    wallet: Arc<dyn BitcoinWallet>,
}

impl ClnGrpcListener {
    pub async fn new(
        config: ClnClientConfig,
        services: Arc<AppServices>,
        wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        let client = ClnGrpcClient::connect(&config).await?;

        Ok(Self {
            client,
            services,
            wallet,
        })
    }

    async fn listen_invoices(&self) -> Result<(), LightningError> {
        let mut next_index = 0_u64;

        loop {
            trace!(next_index, "Waiting for invoice update...");

            let response = self
                .client
                .clone()
                .wait(WaitRequest {
                    subsystem: WaitSubsystem::Invoices as i32,
                    indexname: WaitIndexname::Updated as i32,
                    nextvalue: next_index,
                })
                .await
                .map_err(|e| LightningError::Listener(e.to_string()))?
                .into_inner();

            let updated_index = response.updated;

            if let Some(invoices) = response.invoices {
                match invoices.status() {
                    WaitInvoicesStatus::Paid => {
                        let Some(label) = invoices.label.clone() else {
                            warn!("Invoice update missing label");
                            if let Some(index) = updated_index {
                                next_index = index.saturating_add(1);
                            }
                            continue;
                        };

                        let invoice = self
                            .client
                            .clone()
                            .wait_invoice(WaitinvoiceRequest { label })
                            .await
                            .map_err(|e| LightningError::Listener(e.to_string()))?
                            .into_inner();

                        match invoice.status() {
                            WaitinvoiceStatus::Paid => {
                                if let Err(err) = self.services.event.invoice_paid(invoice.clone().into()).await {
                                    return Err(LightningError::EventProcessing(err.to_string()));
                                }
                            }
                            WaitinvoiceStatus::Expired => {}
                        }
                    }
                    WaitInvoicesStatus::Expired | WaitInvoicesStatus::Unpaid => {}
                }
            } else if next_index != 0 {
                warn!("Invoice wait response missing invoice details");
            }

            if let Some(index) = updated_index {
                next_index = index.saturating_add(1);
            }
        }
    }

    async fn listen_sendpays(&self) -> Result<(), LightningError> {
        let mut next_index = 0_u64;

        loop {
            trace!(next_index, "Waiting for new sendpay update...");

            let response = self
                .client
                .clone()
                .wait(WaitRequest {
                    subsystem: WaitSubsystem::Sendpays as i32,
                    indexname: WaitIndexname::Updated as i32,
                    nextvalue: next_index,
                })
                .await
                .map_err(|e| LightningError::Listener(e.to_string()))?
                .into_inner();

            if let Some(sendpays) = response.sendpays {
                let Some(payment_hash_bytes) = sendpays.payment_hash.clone() else {
                    warn!("Sendpay update missing payment hash");
                    continue;
                };

                let payment_hash = hex::encode(&payment_hash_bytes);
                match sendpays.status() {
                    WaitSendpaysStatus::Complete => {
                        self.handle_sendpay_complete(payment_hash_bytes, sendpays.partid, sendpays.groupid)
                            .await?;
                    }
                    WaitSendpaysStatus::Failed => {
                        let failure_event = LnPayFailureEvent {
                            reason: "Payment failed".to_string(),
                            payment_hash,
                        };
                        if let Err(err) = self.services.event.failed_payment(failure_event).await {
                            return Err(LightningError::EventProcessing(err.to_string()));
                        }
                    }
                    WaitSendpaysStatus::Pending => {}
                }
            }

            if let Some(index) = response.updated {
                next_index = index.saturating_add(1);
            }
        }
    }

    async fn listen_chainmoves(&self) -> Result<(), LightningError> {
        let mut next_index = self
            .services
            .system
            .get_onchain_cursor()
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?
            .map(|c| match c {
                OnchainSyncCursor::CreatedIndex(index) => index,
                _ => 0,
            })
            .unwrap_or(0);

        loop {
            trace!(next_index, "Waiting for new chainmove...");

            let response = self
                .client
                .clone()
                .wait(WaitRequest {
                    subsystem: WaitSubsystem::Chainmoves as i32,
                    indexname: WaitIndexname::Created as i32,
                    nextvalue: next_index,
                })
                .await
                .map_err(|e| LightningError::Listener(e.to_string()))?
                .into_inner();

            if response.created.is_none() {
                warn!("Chainmoves wait response missing created index");
                continue;
            }

            // Replay from the persisted cursor, not the tip `wait` returned: listing from the tip
            // would skip chainmoves between the cursor and the tip.
            let max_index = self.handle_chainmoves(next_index).await?;
            next_index = max_index.saturating_add(1);
            self.services
                .system
                .set_onchain_cursor(OnchainSyncCursor::CreatedIndex(next_index))
                .await
                .map_err(|e| LightningError::Listener(e.to_string()))?;
        }
    }

    async fn handle_sendpay_complete(
        &self,
        payment_hash: Vec<u8>,
        partid: Option<u64>,
        groupid: Option<u64>,
    ) -> Result<(), LightningError> {
        let result = {
            let mut client = self.client.clone();
            client
                .wait_send_pay(WaitsendpayRequest {
                    payment_hash,
                    partid,
                    groupid,
                    timeout: None,
                })
                .await
        };

        let response = result
            .map_err(|e| LightningError::Listener(e.message().to_string()))?
            .into_inner();

        let payment_hash = hex::encode(response.payment_hash);
        let amount_sent_msat = response
            .amount_sent_msat
            .as_ref()
            .map(|amount| amount.msat)
            .unwrap_or_default();
        let amount_msat = response
            .amount_msat
            .as_ref()
            .map(|amount| amount.msat)
            .unwrap_or(amount_sent_msat);
        let fees_msat = amount_sent_msat.saturating_sub(amount_msat);
        let payment_preimage = response.payment_preimage.as_ref().map(hex::encode).unwrap_or_default();

        let payment_time = response
            .completed_at
            .and_then(|completed_at| {
                let secs = completed_at as i64;
                let nanos = ((completed_at - secs as f64) * 1e9) as u32;
                Utc.timestamp_opt(secs, nanos).single()
            })
            .or_else(|| Utc.timestamp_opt(response.created_at as i64, 0).single())
            .unwrap_or_else(Utc::now);

        let success_event = LnPaySuccessEvent {
            amount_msat,
            fees_msat,
            payment_hash,
            payment_preimage,
            payment_time,
        };

        self.services
            .event
            .outgoing_payment(success_event)
            .await
            .map_err(|err| LightningError::EventProcessing(err.to_string()))
    }

    async fn handle_chainmoves(&self, start_index: u64) -> Result<u64, LightningError> {
        let mut client = self.client.clone();
        let currency: Currency = self.wallet.network().into();

        let response = client
            .list_chain_moves(ListchainmovesRequest {
                index: Some(ListchainmovesIndex::Created as i32),
                start: Some(start_index),
                limit: None,
            })
            .await
            .map_err(|e| LightningError::Listener(e.message().to_string()))?
            .into_inner();

        let mut max_index = start_index;

        for chainmove in response.chainmoves {
            max_index = max_index.max(chainmove.created_index);

            let primary_tag = chainmove.primary_tag();
            let account_id = chainmove.account_id.as_str();

            let outpoint = chainmove
                .utxo
                .as_ref()
                .map(|utxo| (hex::encode(&utxo.txid), utxo.outnum));

            match (primary_tag, account_id) {
                (ListchainmovesChainmovesPrimaryTag::Deposit, "wallet") => {
                    let Some((txid, outnum)) = outpoint.clone() else {
                        warn!("Deposit chainmove missing outpoint");
                        continue;
                    };

                    // Prefer the bounded unspent set; only on a miss fall back to the unbounded
                    // spent-inclusive query, since a replayed deposit's UTXO may already be spent.
                    let resolved = match self.wallet.get_output(&txid, Some(outnum), None, false).await {
                        Ok(None) => self.wallet.get_output(&txid, Some(outnum), None, true).await,
                        other => other,
                    };
                    let output = match resolved {
                        Ok(Some(output)) => output,
                        Ok(None) => {
                            trace!(%txid, outnum, "Output not found in wallet, skipping chainmove");
                            continue;
                        }
                        Err(err) => {
                            warn!(%err, %txid, outnum, "Failed to get output from wallet");
                            continue;
                        }
                    };

                    // Propagate write failures so the cursor isn't advanced past a deposit we
                    // failed to credit; the supervisor reconnects and reprocesses idempotently.
                    self.services
                        .event
                        .onchain_deposit(output.into(), currency.clone())
                        .await
                        .map_err(|e| LightningError::EventProcessing(e.to_string()))?;
                }
                (ListchainmovesChainmovesPrimaryTag::Withdrawal, "wallet") => {
                    let Some(spending_txid) = chainmove.spending_txid else {
                        warn!("Withdrawal chainmove missing spending_txid");
                        continue;
                    };

                    let event = OnchainWithdrawalEvent {
                        txid: hex::encode(spending_txid),
                        block_height: Some(chainmove.blockheight),
                    };

                    self.services
                        .event
                        .onchain_withdrawal(event)
                        .await
                        .map_err(|e| LightningError::EventProcessing(e.to_string()))?;
                }
                _ => {}
            }
        }

        if max_index > start_index {
            trace!(start_index, max_index, "Processed chainmoves batch");
        }

        Ok(max_index)
    }
}

#[async_trait]
impl EventsListener for ClnGrpcListener {
    async fn listen(&self) -> Result<(), LightningError> {
        tokio::try_join!(self.listen_invoices(), self.listen_sendpays(), self.listen_chainmoves())?;
        Ok(())
    }
}
