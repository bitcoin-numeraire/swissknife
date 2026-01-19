use crate::domains::{
    bitcoin::{BtcNetwork, BitcoinOutputEvent},
    ln_node::{LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent},
};
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use serde_bolt::bitcoin::hashes::hex::ToHex;
use serde_bolt::bitcoin::hashes::{sha256, Hash};

#[derive(Debug, Deserialize)]
pub struct InvoicePayment {
    pub preimage: String,
    pub msat: u64,
}

#[derive(Debug, Deserialize)]
pub struct SendPaySuccess {
    pub amount_msat: u64,
    pub amount_sent_msat: u64,
    pub payment_hash: String,
    pub payment_preimage: String,
    pub completed_at: u64,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SendPayFailure {
    pub message: String,
    pub data: SendPayFailureData,
}

#[derive(Debug, Deserialize)]
pub struct SendPayFailureData {
    pub payment_hash: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChainMovement {
    pub primary_tag: String,
    pub utxo: String,
    pub output_msat: u64,
    pub debit_msat: u64,
    pub timestamp: i64,
    pub blockheight: u32,
}

impl From<InvoicePayment> for LnInvoicePaidEvent {
    fn from(val: InvoicePayment) -> Self {
        let preimage = hex::decode(val.preimage.clone()).expect("should be hex string");
        let payment_hash = sha256::Hash::hash(&preimage).to_hex();
        LnInvoicePaidEvent {
            payment_hash,
            amount_received_msat: val.msat,
            fee_msat: 0,
            payment_time: Utc::now(),
        }
    }
}

impl From<SendPaySuccess> for LnPaySuccessEvent {
    fn from(val: SendPaySuccess) -> Self {
        LnPaySuccessEvent {
            amount_msat: val.amount_msat,
            fees_msat: val.amount_sent_msat - val.amount_msat,
            payment_hash: val.payment_hash,
            payment_preimage: val.payment_preimage,
            payment_time: Utc.timestamp_opt(val.completed_at as i64, 0).unwrap(),
        }
    }
}

impl From<SendPayFailure> for LnPayFailureEvent {
    fn from(val: SendPayFailure) -> Self {
        LnPayFailureEvent {
            reason: val.message,
            payment_hash: val.data.payment_hash,
        }
    }
}

impl From<ChainMovement> for BitcoinOutputEvent {
    fn from(val: ChainMovement) -> Self {
        let parts = val.utxo.split(":").collect::<Vec<&str>>();
        let txid = parts[0].to_string();
        let output_index = parts[1].parse::<u32>().expect("invalid output index");
        let mut fee_sat = None;

        if val.primary_tag == "withdrawal" {
            fee_sat = Some((val.debit_msat - val.output_msat) / 1000);
        }

        BitcoinOutputEvent {
            txid,
            output_index,
            address: None,
            amount_sat: val.output_msat / 1000,
            timestamp: Utc.timestamp_opt(val.timestamp, 0).unwrap(),
            fee_sat,
            block_height: val.blockheight,
            network: BtcNetwork::default(),
        }
    }
}
