use chrono::{TimeZone, Utc};
use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescriptionRef, Currency as LNInvoiceCurrency};

use crate::{
    application::entities::{Currency, Ledger},
    domains::{
        bitcoin::BtcNetwork,
        invoice::{Invoice, LnInvoice},
    },
};

impl From<Bolt11Invoice> for Invoice {
    fn from(val: Bolt11Invoice) -> Self {
        let payee_pubkey: String = match val.payee_pub_key() {
            Some(key) => key.to_string(),
            None => val.recover_payee_pub_key().to_string(),
        };

        let timestamp = Utc
            .timestamp_opt(
                val.duration_since_epoch().as_secs() as i64,
                val.duration_since_epoch().subsec_nanos(),
            )
            .unwrap();

        Invoice {
            ledger: Ledger::Lightning,
            currency: val.currency().into(),
            amount_msat: val.amount_milli_satoshis(),
            timestamp,
            description: match val.description() {
                Bolt11InvoiceDescriptionRef::Direct(msg) => Some(msg.to_string()),
                Bolt11InvoiceDescriptionRef::Hash(_) => None,
            },
            ln_invoice: Some(LnInvoice {
                bolt11: val.to_string(),
                payment_hash: val.payment_hash().to_string(),
                payee_pubkey,
                description_hash: match val.description() {
                    Bolt11InvoiceDescriptionRef::Direct(_) => None,
                    Bolt11InvoiceDescriptionRef::Hash(h) => Some(h.0.to_string()),
                },
                payment_secret: hex::encode(val.payment_secret().0),
                min_final_cltv_expiry_delta: val.min_final_cltv_expiry_delta(),
                expiry: val.expiry_time(),
                expires_at: timestamp + val.expiry_time(),
            }),
            ..Default::default()
        }
    }
}

impl From<LNInvoiceCurrency> for Currency {
    fn from(val: LNInvoiceCurrency) -> Self {
        match val {
            LNInvoiceCurrency::Bitcoin => Currency::Bitcoin,
            LNInvoiceCurrency::Regtest => Currency::Regtest,
            LNInvoiceCurrency::Signet => Currency::Signet,
            LNInvoiceCurrency::BitcoinTestnet => Currency::BitcoinTestnet,
            LNInvoiceCurrency::Simnet => Currency::Simnet,
        }
    }
}

pub fn parse_network(s: &str) -> BtcNetwork {
    match s.to_lowercase().as_str() {
        "bitcoin" | "mainnet" => BtcNetwork::Bitcoin,
        "testnet" | "testnet3" => BtcNetwork::Testnet,
        "testnet4" => BtcNetwork::Testnet4,
        "regtest" => BtcNetwork::Regtest,
        "simnet" => BtcNetwork::Simnet,
        "signet" => BtcNetwork::Signet,
        _ => BtcNetwork::Bitcoin,
    }
}
