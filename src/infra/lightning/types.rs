use chrono::{TimeZone, Utc};
use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Currency as LNInvoiceCurrency};
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::{
    application::entities::{Currency, Ledger},
    domains::invoices::entities::{Invoice, LnInvoice},
};

impl Into<Invoice> for Bolt11Invoice {
    fn into(self) -> Invoice {
        let payee_pubkey: String = match self.payee_pub_key() {
            Some(key) => key.to_hex(),
            None => self.recover_payee_pub_key().to_hex(),
        };

        let timestamp = Utc
            .timestamp_opt(
                self.duration_since_epoch().as_secs() as i64,
                self.duration_since_epoch().subsec_nanos(),
            )
            .unwrap();

        Invoice {
            ledger: Ledger::LIGHTNING,
            currency: self.currency().into(),
            amount_msat: self.amount_milli_satoshis(),
            timestamp,
            description: match self.description() {
                Bolt11InvoiceDescription::Direct(msg) => Some(msg.to_string()),
                Bolt11InvoiceDescription::Hash(_) => None,
            },
            lightning: Some(LnInvoice {
                bolt11: self.to_string(),
                payment_hash: self.payment_hash().to_hex(),
                payee_pubkey,
                description_hash: match self.description() {
                    Bolt11InvoiceDescription::Direct(_) => None,
                    Bolt11InvoiceDescription::Hash(h) => Some(h.0.to_string()),
                },
                payment_secret: self.payment_secret().0.to_hex(),
                min_final_cltv_expiry_delta: self.min_final_cltv_expiry_delta(),
                expiry: self.expiry_time(),
                expires_at: timestamp + self.expiry_time(),
            }),
            ..Default::default()
        }
    }
}

impl Into<Currency> for LNInvoiceCurrency {
    fn into(self) -> Currency {
        match self {
            LNInvoiceCurrency::Bitcoin => Currency::Bitcoin,
            LNInvoiceCurrency::Regtest => Currency::Regtest,
            LNInvoiceCurrency::Signet => Currency::Signet,
            LNInvoiceCurrency::BitcoinTestnet => Currency::BitcoinTestnet,
            LNInvoiceCurrency::Simnet => Currency::Simnet,
        }
    }
}
