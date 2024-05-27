use chrono::{TimeZone, Utc};
use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription};
use serde_bolt::bitcoin::hashes::hex::ToHex;
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
            payment_secret: self.payment_secret.to_hex(),
            timestamp: Utc
                .timestamp_opt(
                    invoice.duration_since_epoch().as_secs() as i64,
                    invoice.duration_since_epoch().subsec_nanos(),
                )
                .unwrap(),
            expiry: invoice.expiry_time(),
            expires_at: Utc
                .timestamp_opt(
                    invoice.duration_until_expiry().as_secs() as i64,
                    invoice.duration_until_expiry().subsec_nanos(),
                )
                .unwrap(),
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
            ..Default::default()
        }
    }
}
