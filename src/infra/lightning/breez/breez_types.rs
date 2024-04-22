use breez_sdk_core::{LNInvoice, Payment};

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
            payment_secret: self.payment_secret,
            min_final_cltv_expiry_delta: self.min_final_cltv_expiry_delta,
            timestamp: self.timestamp,
            expiry: self.expiry,
            ..Default::default()
        }
    }
}

impl Into<LightningPayment> for Payment {
    fn into(self) -> LightningPayment {
        LightningPayment {
            payment_hash: self.id,
            error: self.error,
            amount_msat: self.amount_msat,
            fee_msat: Some(self.fee_msat),
            payment_time: Some(self.payment_time),
            description: self.description,
            metadata: self.metadata,
            ..Default::default()
        }
    }
}
