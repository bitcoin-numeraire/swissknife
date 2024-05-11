use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription};
use serde_bolt::bitcoin::hashes::hex::ToHex;
use serde_json::json;
use std::str::FromStr;

use super::cln::InvoiceResponse;
use crate::domains::lightning::entities::LightningInvoice;

impl Into<LightningInvoice> for InvoiceResponse {
    fn into(self) -> LightningInvoice {
        let invoice = Bolt11Invoice::from_str(self.bolt11.as_str()).unwrap(); // Cannot fail by assertion

        let payee_pubkey: String = match invoice.payee_pub_key() {
            Some(key) => key.to_hex(),
            None => invoice.recover_payee_pub_key().to_hex(),
        };

        LightningInvoice {
            bolt11: self.bolt11,
            payment_hash: invoice.payment_hash().to_hex(),
            amount_msat: invoice.amount_milli_satoshis(),
            payment_secret: self.payment_secret,
            timestamp: invoice.duration_since_epoch().as_secs(),
            expiry: invoice.expiry_time().as_secs(),
            network: invoice.network().to_string(),
            payee_pubkey,
            description: match invoice.description() {
                Bolt11InvoiceDescription::Direct(msg) => Some(msg.to_string()),
                Bolt11InvoiceDescription::Hash(_) => None,
            },
            description_hash: match invoice.description() {
                Bolt11InvoiceDescription::Direct(_) => None,
                Bolt11InvoiceDescription::Hash(h) => Some(h.0.to_string()),
            },
            min_final_cltv_expiry_delta: invoice.min_final_cltv_expiry_delta(),
            details: Some(json!({
                "created_index": self.created_index,
                "warning_capacity": self.warning_capacity,
                "warning_deadends": self.warning_deadends,
                "warning_mpp": self.warning_mpp,
                "warning_offline": self.warning_offline,
                "warning_private_unused": self.warning_private_unused,
            })),
            ..Default::default()
        }
    }
}
