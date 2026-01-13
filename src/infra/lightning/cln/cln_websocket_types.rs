use crate::domains::{
    bitcoin::{BitcoinNetwork, BitcoinOutputEvent},
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

#[derive(Debug, Deserialize)]
pub struct CoinMovement {
    #[serde(alias = "utxo_txid")]
    pub txid: Option<String>,
    #[serde(alias = "output", alias = "utxo_outnum")]
    pub output_index: Option<u32>,
    pub address: Option<String>,
    #[serde(alias = "type")]
    pub movement_type: Option<String>,
    pub primary_tag: Option<String>,
    #[serde(alias = "amount_msat")]
    pub amount_msat: Option<String>,
    #[serde(alias = "credit_msat")]
    pub credit_msat: Option<String>,
    #[serde(alias = "debit_msat")]
    pub debit_msat: Option<String>,
    #[serde(alias = "fees_msat")]
    pub fees_msat: Option<String>,
    pub timestamp: Option<u64>,
    pub blockheight: Option<u32>,
    pub tag: Option<String>,
}

impl CoinMovement {
    fn parse_msat(value: &Option<String>) -> Option<i64> {
        value
            .as_ref()
            .and_then(|raw| raw.trim_end_matches("msat").parse::<i64>().ok())
    }

    fn amount_msat(&self) -> Option<i64> {
        Self::parse_msat(&self.amount_msat)
            .or_else(|| Self::parse_msat(&self.credit_msat))
            .or_else(|| Self::parse_msat(&self.debit_msat))
    }

    fn fee_sat(&self) -> Option<i64> {
        Self::parse_msat(&self.fees_msat).map(|fee| fee / 1000)
    }

    pub fn is_chain_movement(&self) -> bool {
        matches!(self.movement_type.as_deref(), Some("chain_mvt"))
    }

    pub fn movement_tag(&self) -> Option<&str> {
        self.primary_tag.as_deref().or(self.tag.as_deref())
    }
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

impl TryFrom<CoinMovement> for BitcoinOutputEvent {
    type Error = &'static str;

    fn try_from(val: CoinMovement) -> Result<Self, Self::Error> {
        let txid = val.txid.clone().ok_or("missing txid")?;
        let output_index = val.output_index.ok_or("missing output index")?;
        let amount_msat = val.amount_msat().ok_or("missing amount")?;
        let amount_sat = amount_msat.unsigned_abs() / 1000;

        Ok(BitcoinOutputEvent {
            txid,
            output_index,
            address: val.address.clone(),
            amount_sat,
            timestamp: val.timestamp.and_then(|t| Utc.timestamp_opt(t as i64, 0).single()),
            fee_sat: val.fee_sat().map(|fee| fee.unsigned_abs()),
            block_height: val.blockheight,
            confirmations: None,
            network: BitcoinNetwork::default(),
        })
    }
}
