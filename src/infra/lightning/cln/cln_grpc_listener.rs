use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;
use tokio::time::sleep;
use tonic::{transport::Channel, Code};
use tracing::{error, trace, warn};

use crate::{
    application::{entities::Currency, errors::LightningError},
    domains::{
        bitcoin::BitcoinWallet,
        event::{EventUseCases, LnPayFailureEvent, LnPaySuccessEvent, OnchainWithdrawalEvent},
    },
    infra::lightning::EventsListener,
};

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

pub struct ClnGrpcListener {
    client: NodeClient<Channel>,
    events: Arc<dyn EventUseCases>,
    wallet: Arc<dyn BitcoinWallet>,
    retry_delay: Duration,
}

impl ClnGrpcListener {
    pub async fn new(
        config: ClnClientConfig,
        events: Arc<dyn EventUseCases>,
        wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<Self, LightningError> {
        let client = ClnGrpcClient::connect(&config).await?;

        Ok(Self {
            client,
            events,
            wallet,
            retry_delay: config.retry_delay,
        })
    }

    /// Handles a gRPC error by sleeping and returning Ok(()) for retryable errors,
    /// or returning Err for fatal errors.
    async fn handle_grpc_error(&self, err: tonic::Status, context: &str) -> Result<(), LightningError> {
        match err.code() {
            Code::Aborted
            | Code::Cancelled
            | Code::DeadlineExceeded
            | Code::Internal
            | Code::FailedPrecondition
            | Code::Unavailable => {
                error!(err = err.message(), "{}. Retrying...", context);
                sleep(self.retry_delay).await;
                Ok(())
            }
            _ => Err(LightningError::Listener(err.to_string())),
        }
    }

    async fn listen_invoices(&self) -> Result<(), LightningError> {
        let mut next_index = 0_u64;

        loop {
            trace!(next_index, "Waiting for invoice update...");

            let result = {
                let mut client = self.client.clone();
                client
                    .wait(WaitRequest {
                        subsystem: WaitSubsystem::Invoices as i32,
                        indexname: WaitIndexname::Updated as i32,
                        nextvalue: next_index,
                    })
                    .await
            };

            match result {
                Ok(response) => {
                    let response = response.into_inner();
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

                                let invoice = loop {
                                    let result = {
                                        let mut client = self.client.clone();
                                        client.wait_invoice(WaitinvoiceRequest { label: label.clone() }).await
                                    };

                                    match result {
                                        Ok(response) => break response.into_inner(),
                                        Err(err) => {
                                            self.handle_grpc_error(err, "Error waiting for invoice").await?;
                                        }
                                    }
                                };

                                match invoice.status() {
                                    WaitinvoiceStatus::Paid => {
                                        if let Err(err) = self.events.invoice_paid(invoice.clone().into()).await {
                                            warn!(%err, "Failed to process incoming payment");
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
                Err(err) => {
                    self.handle_grpc_error(err, "Error waiting for invoice").await?;
                }
            }
        }
    }

    async fn listen_sendpays(&self) -> Result<(), LightningError> {
        let mut next_index = 0_u64;

        loop {
            trace!(next_index, "Waiting for new sendpay update...");

            let result = {
                let mut client = self.client.clone();
                client
                    .wait(WaitRequest {
                        subsystem: WaitSubsystem::Sendpays as i32,
                        indexname: WaitIndexname::Updated as i32,
                        nextvalue: next_index,
                    })
                    .await
            };

            match result {
                Ok(response) => {
                    let response = response.into_inner();

                    if let Some(sendpays) = response.sendpays {
                        let Some(payment_hash_bytes) = sendpays.payment_hash.clone() else {
                            warn!("Sendpay update missing payment hash");
                            continue;
                        };

                        let payment_hash = hex::encode(&payment_hash_bytes);
                        match sendpays.status() {
                            WaitSendpaysStatus::Complete => {
                                if let Err(err) = self
                                    .handle_sendpay_complete(payment_hash_bytes, sendpays.partid, sendpays.groupid)
                                    .await
                                {
                                    warn!(%err, "Failed to process completed outgoing payment");
                                }
                            }
                            WaitSendpaysStatus::Failed => {
                                let failure_event = LnPayFailureEvent {
                                    reason: "Payment failed".to_string(),
                                    payment_hash,
                                };
                                if let Err(err) = self.events.failed_payment(failure_event).await {
                                    warn!(%err, "Failed to process failed outgoing payment");
                                }
                            }
                            WaitSendpaysStatus::Pending => {}
                        }
                    }

                    let updated_index = response.updated;
                    if let Some(index) = updated_index {
                        next_index = index.saturating_add(1);
                    }
                }
                Err(err) => {
                    self.handle_grpc_error(err, "Error waiting for sendpay updates").await?;
                }
            }
        }
    }

    async fn listen_chainmoves(&self) -> Result<(), LightningError> {
        let mut next_index = 0_u64;

        loop {
            trace!(next_index, "Waiting for new chainmove...");

            let result = {
                let mut client = self.client.clone();
                client
                    .wait(WaitRequest {
                        subsystem: WaitSubsystem::Chainmoves as i32,
                        indexname: WaitIndexname::Created as i32,
                        nextvalue: next_index,
                    })
                    .await
            };

            match result {
                Ok(response) => {
                    let response = response.into_inner();
                    let Some(created_index) = response.created else {
                        warn!("Chainmoves wait response missing created index");
                        continue;
                    };

                    match self.handle_chainmoves(created_index).await {
                        Ok(max_index) => {
                            next_index = max_index.saturating_add(1);
                        }
                        Err(err) => {
                            warn!(%err, "Failed to process onchain chainmoves");
                        }
                    }
                }
                Err(err) => {
                    self.handle_grpc_error(err, "Error waiting for chainmoves").await?;
                }
            }
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

        self.events
            .outgoing_payment(success_event)
            .await
            .map_err(|err| LightningError::Listener(err.to_string()))
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
            let originating_account = chainmove.originating_account.as_deref();

            let outpoint = chainmove
                .utxo
                .as_ref()
                .map(|utxo| (hex::encode(&utxo.txid), utxo.outnum));

            match (primary_tag, account_id, originating_account) {
                (ListchainmovesChainmovesPrimaryTag::Deposit, "wallet", _) => {
                    let Some((txid, outnum)) = outpoint.clone() else {
                        warn!("Deposit chainmove missing outpoint");
                        continue;
                    };

                    let output = match self.wallet.get_output(&txid, Some(outnum), None, false).await {
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

                    if let Err(err) = self.events.onchain_deposit(output.into(), currency.clone()).await {
                        warn!(%err, "Failed to process onchain deposit");
                    }
                }
                (ListchainmovesChainmovesPrimaryTag::Deposit, "external", Some("wallet"))
                | (ListchainmovesChainmovesPrimaryTag::Withdrawal, "wallet", _) => {
                    let Some(spending_txid) = chainmove.spending_txid else {
                        warn!("Withdrawal chainmove missing spending_txid");
                        continue;
                    };

                    let event = OnchainWithdrawalEvent {
                        txid: spending_txid.to_hex(),
                        block_height: Some(chainmove.blockheight),
                    };

                    if let Err(err) = self.events.onchain_withdrawal(event).await {
                        warn!(%err, "Failed to process onchain withdrawal");
                    }
                }
                _ => {
                    trace!(
                        primary_tag = ?primary_tag,
                        account_id = account_id,
                        originating_account = originating_account.unwrap_or_default(),
                        "Unsupported chainmove event"
                    );
                }
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
