use std::time::Duration;

use breez_sdk_core::{LNInvoice, Network as BreezNetwork, Payment as BreezPayment};
use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::{
    application::entities::{Currency, Ledger, Network},
    domains::{
        invoices::entities::{Invoice, LnInvoice},
        payments::entities::Payment,
    },
};

impl Into<Invoice> for LNInvoice {
    fn into(self) -> Invoice {
        Invoice {
            ledger: Ledger::LIGHTNING,
            currency: Currency::BTC,
            description: self.description,
            amount_msat: self.amount_msat,
            timestamp: Utc.timestamp_opt(self.timestamp as i64, 0).unwrap(),
            lightning: Some(LnInvoice {
                bolt11: self.bolt11,
                payee_pubkey: self.payee_pubkey,
                payment_hash: self.payment_hash,
                description_hash: self.description_hash,
                payment_secret: self.payment_secret.to_hex(),
                min_final_cltv_expiry_delta: self.min_final_cltv_expiry_delta,
                network: self.network.into(),
                expiry: Duration::from_secs(self.expiry),
                expires_at: Utc
                    .timestamp_opt((self.timestamp + self.expiry) as i64, 0)
                    .unwrap(),
            }),
            ..Default::default()
        }
    }
}

impl Into<Payment> for BreezPayment {
    fn into(self) -> Payment {
        Payment {
            ledger: Ledger::LIGHTNING,
            currency: Currency::BTC,
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

impl Into<Network> for BreezNetwork {
    fn into(self) -> Network {
        match self {
            BreezNetwork::Bitcoin => Network::Bitcoin,
            BreezNetwork::Regtest => Network::Regtest,
            BreezNetwork::Signet => Network::Signet,
            BreezNetwork::Testnet => Network::BitcoinTestnet,
        }
    }
}
