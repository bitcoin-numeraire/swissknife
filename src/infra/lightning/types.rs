use bitcoin::{Address, Network};
use chrono::{TimeZone, Utc};
use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Currency as LNInvoiceCurrency};
use serde_bolt::bitcoin::hashes::hex::ToHex;
use std::str::FromStr;

use crate::{
    application::entities::{Currency, Ledger},
    domains::invoice::{Invoice, LnInvoice},
};

impl From<Bolt11Invoice> for Invoice {
    fn from(val: Bolt11Invoice) -> Self {
        let payee_pubkey: String = match val.payee_pub_key() {
            Some(key) => key.to_hex(),
            None => val.recover_payee_pub_key().to_hex(),
        };

        let timestamp = Utc
            .timestamp_opt(
                val.duration_since_epoch().as_secs() as i64,
                val.duration_since_epoch().subsec_nanos(),
            )
            .unwrap();

        Invoice {
            ledger: Ledger::Lightning,
            amount_msat: val.amount_milli_satoshis(),
            timestamp,
            description: match val.description() {
                Bolt11InvoiceDescription::Direct(msg) => Some(msg.to_string()),
                Bolt11InvoiceDescription::Hash(_) => None,
            },
            ln_invoice: Some(LnInvoice {
                bolt11: val.to_string(),
                payment_hash: val.payment_hash().to_hex(),
                payee_pubkey,
                description_hash: match val.description() {
                    Bolt11InvoiceDescription::Direct(_) => None,
                    Bolt11InvoiceDescription::Hash(h) => Some(h.0.to_string()),
                },
                payment_secret: val.payment_secret().0.to_hex(),
                min_final_cltv_expiry_delta: val.min_final_cltv_expiry_delta(),
                expiry: val.expiry_time(),
                expires_at: timestamp + val.expiry_time(),
            }),
            ..Default::default()
        }
    }
}

pub fn currency_from_network_name(name: &str) -> Option<Currency> {
    match name.to_lowercase().as_str() {
        "bitcoin" | "mainnet" => Some(Currency::Bitcoin),
        "testnet" | "testnet3" => Some(Currency::BitcoinTestnet),
        "regtest" => Some(Currency::Regtest),
        "signet" => Some(Currency::Signet),
        "simnet" => Some(Currency::Simnet),
        _ => None,
    }
}

pub fn currency_to_bitcoin_network(currency: Currency) -> Option<Network> {
    match currency {
        Currency::Bitcoin => Some(Network::Bitcoin),
        Currency::BitcoinTestnet => Some(Network::Testnet),
        Currency::Regtest | Currency::Simnet => Some(Network::Regtest),
        Currency::Signet => Some(Network::Signet),
    }
}

pub fn validate_address_for_currency(address: &str, currency: Currency) -> bool {
    if let Ok(parsed) = Address::from_str(address) {
        if let Some(expected_network) = currency_to_bitcoin_network(currency) {
            return parsed.require_network(expected_network).is_ok();
        }
    }

    false
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
