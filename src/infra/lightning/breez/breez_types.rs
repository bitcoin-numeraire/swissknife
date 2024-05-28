use std::time::Duration;

use breez_sdk_core::{LNInvoice, Payment};
use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::domains::lightning::entities::{
    Invoice, InvoiceType, LightningInvoice, LightningPayment, PaymentType,
};

impl Into<Invoice> for LNInvoice {
    fn into(self) -> Invoice {
        Invoice {
            invoice_type: InvoiceType::LIGHTNING,
            network: self.network.to_string(),
            description: self.description,
            amount_msat: self.amount_msat,
            timestamp: Utc.timestamp_opt(self.timestamp as i64, 0).unwrap(),
            expiry: Duration::from_secs(self.expiry),
            expires_at: Utc
                .timestamp_opt((self.timestamp + self.expiry) as i64, 0)
                .unwrap(),
            lightning: Some(LightningInvoice {
                bolt11: self.bolt11,
                payee_pubkey: self.payee_pubkey,
                payment_hash: self.payment_hash,
                description_hash: self.description_hash,
                payment_secret: self.payment_secret.to_hex(),
                min_final_cltv_expiry_delta: self.min_final_cltv_expiry_delta,
            }),
            ..Default::default()
        }
    }
}

impl Into<LightningPayment> for Payment {
    fn into(self) -> LightningPayment {
        LightningPayment {
            payment_type: PaymentType::LIGHTNING,
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
