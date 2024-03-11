use breez_sdk_core::{LNInvoice, Payment};

use crate::domains::lightning::entities::{LightningInvoice, LightningPayment};

impl Into<LightningInvoice> for LNInvoice {
    fn into(self) -> LightningInvoice {
        LightningInvoice {
            id: None,
            lightning_address: None,
            bolt11: self.bolt11,
            network: self.network.to_string(),
            payee_pubkey: self.payee_pubkey,
            payment_hash: self.payment_hash,
            description: self.description,
            description_hash: self.description_hash,
            amount_msat: self.amount_msat.map(|amt| amt as i64),
            payment_secret: self.payment_secret,
            min_final_cltv_expiry_delta: self.min_final_cltv_expiry_delta as i64,
            timestamp: self.timestamp as i64,
            expiry: self.expiry as i64,
            status: "PENDING".to_string(),
            fee_msat: None,
            payment_time: None,
            created_at: None,
            updated_at: None,
        }
    }
}

impl Into<LightningPayment> for Payment {
    fn into(self) -> LightningPayment {
        LightningPayment {
            id: None,
            payment_hash: self.id,
            lightning_address: None,
            error: self.error,
            amount_msat: self.amount_msat as i64,
            fee_msat: Some(self.fee_msat as i64),
            payment_time: Some(self.payment_time),
            status: if self.error.is_some() {
                "FAILED".to_string()
            } else {
                "PENDING".to_string()
            },
            description: self.description,
            metadata: self.metadata,
            details: Some(self.details),
            created_at: None,
            updated_at: None,
        }
    }
}
