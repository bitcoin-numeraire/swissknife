use std::time::Duration;

use breez_sdk_core::{LNInvoice, Payment};
use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::domains::lightning::entities::{LightningInvoice, LightningPayment};

impl Into<LightningInvoice> for LNInvoice {
    fn into(self) -> LightningInvoice {
        LightningInvoice {
            bolt11: self.bolt11,
            network: self.network.to_string(),
            payee_pubkey: self.payee_pubkey,
            payment_hash: self.payment_hash,
            description: self.description,
            description_hash: self.description_hash,
            amount_msat: self.amount_msat,
            payment_secret: self.payment_secret.to_hex(),
            min_final_cltv_expiry_delta: self.min_final_cltv_expiry_delta,
            timestamp: Utc.timestamp_opt(self.timestamp as i64, 0).unwrap(),
            expiry: Duration::from_secs(self.expiry),
            expires_at: Utc
                .timestamp_opt((self.timestamp + self.expiry) as i64, 0)
                .unwrap(),
            ..Default::default()
        }
    }
}

impl Into<LightningPayment> for Payment {
    fn into(self) -> LightningPayment {
        LightningPayment {
            payment_hash: Some(self.id),
            error: self.error,
            amount_msat: self.amount_msat,
            fee_msat: Some(self.fee_msat),
            payment_time: Some(Utc.timestamp_opt(self.payment_time, 0).unwrap()),
            description: self.description,
            metadata: self.metadata,
            ..Default::default()
        }
    }
}
