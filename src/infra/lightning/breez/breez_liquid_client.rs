use serde::{Deserialize, Serialize};
use std::sync::Arc;

use async_trait::async_trait;
use breez_sdk_liquid::{
    model::{
        ConnectRequest, GetPaymentRequest, LiquidNetwork, ListPaymentDetails, ListPaymentsRequest, PayAmount,
        PayOnchainRequest, PaymentDetails, PaymentMethod, PaymentState, PaymentType, PreparePayOnchainRequest,
        PreparePayOnchainResponse, PrepareReceiveRequest, PrepareSendRequest, ReceiveAmount, ReceivePaymentRequest,
        SendPaymentRequest,
    },
    sdk::LiquidSdk,
};

use crate::{
    application::errors::{BitcoinError, LightningError},
    domains::{
        bitcoin::{
            BitcoinWallet, BtcAddressType, BtcNetwork, BtcOutput, BtcPreparedTransaction, BtcTransaction,
            OnchainSyncBatch, OnchainSyncCursor,
        },
        invoice::Invoice,
        payment::Payment,
        system::HealthStatus,
    },
    infra::lightning::{breez::BreezListener, LnClient},
};

#[derive(Clone, Debug, Deserialize)]
pub struct BreezClientConfig {
    pub api_key: String,
    pub working_dir: String,
    pub mnemonic: String,
    pub passphrase: Option<String>,
    pub log_in_file: bool,
    pub network: String,
}

pub struct BreezClient {
    sdk: Arc<LiquidSdk>,
    network: BtcNetwork,
}

#[derive(Debug, Deserialize, Serialize)]
struct BreezPreparedTransaction {
    address: String,
    receiver_amount_sat: u64,
    claim_fees_sat: u64,
    total_fees_sat: u64,
}

impl BreezClient {
    pub async fn new(config: BreezClientConfig, listener: BreezListener) -> Result<Self, LightningError> {
        if config.log_in_file {
            LiquidSdk::init_logging(&config.working_dir, None).map_err(|e| LightningError::Logging(e.to_string()))?;
        }

        let network = match config.network.to_lowercase().as_str() {
            "bitcoin" => (LiquidNetwork::Mainnet, BtcNetwork::Bitcoin),
            "testnet" => (LiquidNetwork::Testnet, BtcNetwork::Testnet),
            "regtest" => (LiquidNetwork::Regtest, BtcNetwork::Regtest),
            _ => return Err(LightningError::ParseConfig("Invalid network".to_string())),
        };

        let mut sdk_config = LiquidSdk::default_config(network.0, Some(config.api_key.clone()))
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;
        sdk_config.working_dir = config.working_dir.clone();

        let sdk = LiquidSdk::connect(ConnectRequest {
            config: sdk_config,
            mnemonic: Some(config.mnemonic.clone()),
            passphrase: config.passphrase,
            seed: None,
        })
        .await
        .map_err(|e| LightningError::Connect(e.to_string()))?;

        sdk.add_event_listener(Box::new(listener))
            .await
            .map_err(|e| LightningError::Listener(e.to_string()))?;

        Ok(Self {
            sdk,
            network: network.1,
        })
    }
}

#[async_trait]
impl LnClient for BreezClient {
    async fn disconnect(&self) -> Result<(), LightningError> {
        self.sdk
            .disconnect()
            .await
            .map_err(|e| LightningError::Disconnect(e.to_string()))
    }

    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        label: String,
        expiry: u32,
        deschashonly: bool,
    ) -> Result<Invoice, LightningError> {
        let payer_amount_sat = (amount_msat / 1000).max(1);
        let prepare = self
            .sdk
            .prepare_receive_payment(&PrepareReceiveRequest {
                payment_method: PaymentMethod::Bolt11Invoice,
                amount: Some(ReceiveAmount::Bitcoin { payer_amount_sat }),
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let response = self
            .sdk
            .receive_payment(&ReceivePaymentRequest {
                prepare_response: prepare,
                description: Some(description),
                use_description_hash: Some(deschashonly),
                payer_note: if label.is_empty() { None } else { Some(label) },
            })
            .await
            .map_err(|e| LightningError::Invoice(e.to_string()))?;

        let parsed =
            LiquidSdk::parse_invoice(&response.destination).map_err(|e| LightningError::Invoice(e.to_string()))?;
        let mut invoice: Invoice = parsed.into();
        if expiry > 0 {
            invoice.ln_invoice = invoice.ln_invoice.map(|mut ln| {
                ln.expiry = std::time::Duration::from_secs(expiry as u64);
                ln
            });
        }
        Ok(invoice)
    }

    async fn pay(&self, bolt11: String, amount_msat: Option<u64>, label: String) -> Result<Payment, LightningError> {
        let prepare = self
            .sdk
            .prepare_send_payment(&PrepareSendRequest {
                destination: bolt11,
                amount: amount_msat.map(|msat| PayAmount::Bitcoin {
                    receiver_amount_sat: (msat / 1000).max(1),
                }),
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        let response = self
            .sdk
            .send_payment(&SendPaymentRequest {
                prepare_response: prepare,
                use_asset_fees: None,
                payer_note: if label.is_empty() { None } else { Some(label) },
            })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.payment.into())
    }

    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError> {
        let response = self
            .sdk
            .get_payment(&GetPaymentRequest::PaymentHash { payment_hash })
            .await
            .map_err(|e| LightningError::InvoiceByHash(e.to_string()))?;

        Ok(response.map(Into::into))
    }

    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError> {
        let response = self
            .sdk
            .get_payment(&GetPaymentRequest::PaymentHash { payment_hash })
            .await
            .map_err(|e| LightningError::Pay(e.to_string()))?;

        Ok(response.map(Into::into))
    }

    async fn cancel_invoice(&self, _payment_hash: String, _label: String) -> Result<(), LightningError> {
        Err(LightningError::CancelInvoice(
            "Invoice cancellation is not supported for Breez".to_string(),
        ))
    }

    async fn health(&self) -> Result<HealthStatus, LightningError> {
        self.sdk
            .get_info()
            .await
            .map_err(|e| LightningError::HealthCheck(e.to_string()))?;
        Ok(HealthStatus::Operational)
    }
}

#[async_trait]
impl BitcoinWallet for BreezClient {
    async fn new_address(&self, _address_type: BtcAddressType) -> Result<String, BitcoinError> {
        Err(BitcoinError::Unsupported(
            "Breez Liquid does not expose a direct Bitcoin deposit address API".to_string(),
        ))
    }

    async fn prepare_transaction(
        &self,
        address: String,
        amount_sat: u64,
        fee_rate: Option<u32>,
    ) -> Result<BtcPreparedTransaction, BitcoinError> {
        let prepare_response = self
            .sdk
            .prepare_pay_onchain(&PreparePayOnchainRequest {
                amount: PayAmount::Bitcoin {
                    receiver_amount_sat: amount_sat,
                },
                fee_rate_sat_per_vbyte: fee_rate,
            })
            .await
            .map_err(|e| BitcoinError::PrepareTransaction(e.to_string()))?;

        let serialized = serde_json::to_string(&BreezPreparedTransaction {
            address,
            receiver_amount_sat: prepare_response.receiver_amount_sat,
            claim_fees_sat: prepare_response.claim_fees_sat,
            total_fees_sat: prepare_response.total_fees_sat,
        })
        .map_err(|e| BitcoinError::PrepareTransaction(e.to_string()))?;

        Ok(BtcPreparedTransaction {
            txid: format!("breez-onchain-{}", prepare_response.receiver_amount_sat),
            fee_sat: prepare_response.total_fees_sat,
            psbt: serialized,
            locked_utxos: Vec::new(),
        })
    }

    async fn sign_send_transaction(&self, prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        let prepared_data: BreezPreparedTransaction =
            serde_json::from_str(&prepared.psbt).map_err(|e| BitcoinError::FinalizeTransaction(e.to_string()))?;

        self.sdk
            .pay_onchain(&PayOnchainRequest {
                address: prepared_data.address,
                prepare_response: PreparePayOnchainResponse {
                    receiver_amount_sat: prepared_data.receiver_amount_sat,
                    claim_fees_sat: prepared_data.claim_fees_sat,
                    total_fees_sat: prepared_data.total_fees_sat,
                },
            })
            .await
            .map_err(|e| BitcoinError::BroadcastTransaction(e.to_string()))?;

        Ok(())
    }

    async fn release_prepared_transaction(&self, _prepared: &BtcPreparedTransaction) -> Result<(), BitcoinError> {
        Ok(())
    }

    async fn get_transaction(&self, txid: &str) -> Result<Option<BtcTransaction>, BitcoinError> {
        let payments = self
            .sdk
            .list_payments(&ListPaymentsRequest {
                details: Some(ListPaymentDetails::Bitcoin { address: None }),
                ..Default::default()
            })
            .await
            .map_err(|e| BitcoinError::GetTransaction(e.to_string()))?;

        let payment = payments.into_iter().find(|payment| {
            payment.tx_id.as_deref() == Some(txid)
                || matches!(
                    &payment.details,
                    PaymentDetails::Bitcoin {
                        lockup_tx_id,
                        claim_tx_id,
                        refund_tx_id,
                        ..
                    } if lockup_tx_id.as_deref() == Some(txid)
                        || claim_tx_id.as_deref() == Some(txid)
                        || refund_tx_id.as_deref() == Some(txid)
                )
        });

        Ok(payment.and_then(|payment| {
            let payment_type = payment.payment_type;
            let amount_sat = payment.amount_sat;
            let txid_for_payment = payment.tx_id.clone();

            match payment.details {
                PaymentDetails::Bitcoin {
                    lockup_tx_id,
                    claim_tx_id,
                    refund_tx_id,
                    bitcoin_address,
                    ..
                } => {
                    let resolved_txid = txid_for_payment
                        .or(lockup_tx_id)
                        .or(claim_tx_id)
                        .or(refund_tx_id)
                        .unwrap_or_else(|| txid.to_string());

                    Some(BtcTransaction {
                        txid: resolved_txid,
                        block_height: None,
                        outputs: vec![crate::domains::bitcoin::BtcTransactionOutput {
                            output_index: 0,
                            address: bitcoin_address,
                            amount_sat,
                            is_ours: payment_type == PaymentType::Receive,
                        }],
                        is_outgoing: payment_type == PaymentType::Send,
                    })
                }
                _ => None,
            }
        }))
    }

    async fn synchronize(&self, cursor: Option<OnchainSyncCursor>) -> Result<OnchainSyncBatch, BitcoinError> {
        let offset = match cursor {
            Some(OnchainSyncCursor::CreatedIndex(index)) => index as u32,
            _ => 0,
        };

        let payments = self
            .sdk
            .list_payments(&ListPaymentsRequest {
                offset: Some(offset),
                limit: Some(100),
                details: Some(ListPaymentDetails::Bitcoin { address: None }),
                sort_ascending: Some(true),
                ..Default::default()
            })
            .await
            .map_err(|e| BitcoinError::Synchronize(e.to_string()))?;

        let mut events = Vec::new();
        for payment in &payments {
            let (txid, address) = match &payment.details {
                PaymentDetails::Bitcoin {
                    lockup_tx_id,
                    claim_tx_id,
                    refund_tx_id,
                    bitcoin_address,
                    ..
                } => (
                    payment
                        .tx_id
                        .clone()
                        .or_else(|| lockup_tx_id.clone())
                        .or_else(|| claim_tx_id.clone())
                        .or_else(|| refund_tx_id.clone()),
                    bitcoin_address.clone(),
                ),
                _ => continue,
            };

            let Some(txid) = txid else {
                continue;
            };

            match payment.payment_type {
                PaymentType::Receive => events.push(crate::domains::bitcoin::OnchainTransaction::Deposit(BtcOutput {
                    txid: txid.clone(),
                    output_index: 0,
                    address,
                    amount_sat: payment.amount_sat,
                    outpoint: format!("{}:{}", txid, 0),
                    status: if payment.status == PaymentState::Complete {
                        crate::domains::bitcoin::BtcOutputStatus::Confirmed
                    } else {
                        crate::domains::bitcoin::BtcOutputStatus::Unconfirmed
                    },
                    ..Default::default()
                })),
                PaymentType::Send => events.push(crate::domains::bitcoin::OnchainTransaction::Withdrawal(
                    crate::domains::event::OnchainWithdrawalEvent {
                        txid,
                        block_height: None,
                    },
                )),
            }
        }

        let next_cursor = if payments.is_empty() {
            None
        } else {
            Some(OnchainSyncCursor::CreatedIndex(offset as u64 + payments.len() as u64))
        };

        Ok(OnchainSyncBatch { events, next_cursor })
    }

    async fn get_output(
        &self,
        txid: &str,
        output_index: Option<u32>,
        address: Option<&str>,
        _include_spent: bool,
    ) -> Result<Option<BtcOutput>, BitcoinError> {
        let Some(transaction) = self.get_transaction(txid).await? else {
            return Ok(None);
        };

        let output = transaction.outputs.iter().find(|output| match output_index {
            Some(index) => output.output_index == index,
            None => address.map(|target| output.address == target).unwrap_or(false),
        });

        Ok(output.map(|output| BtcOutput {
            txid: transaction.txid.clone(),
            output_index: output.output_index,
            address: output.address.clone(),
            amount_sat: output.amount_sat,
            outpoint: format!("{}:{}", transaction.txid, output.output_index),
            ..Default::default()
        }))
    }

    fn network(&self) -> BtcNetwork {
        self.network
    }
}
