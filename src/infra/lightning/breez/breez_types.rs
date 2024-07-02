use std::time::Duration;

use breez_sdk_core::{
    LNInvoice, Network as BreezNetwork, Payment as BreezPayment, PaymentDetails, PaymentFailedData,
};
use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::{
    application::entities::{Currency, Ledger},
    domains::{
        invoices::entities::{Invoice, LnInvoice},
        lightning::entities::{LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent},
        payments::entities::Payment,
    },
};

impl From<LNInvoice> for Invoice {
    fn from(val: LNInvoice) -> Self {
        Invoice {
            ledger: Ledger::Lightning,
            currency: val.network.into(),
            description: val.description,
            amount_msat: val.amount_msat,
            timestamp: Utc.timestamp_opt(val.timestamp as i64, 0).unwrap(),
            lightning: Some(LnInvoice {
                bolt11: val.bolt11,
                payee_pubkey: val.payee_pubkey,
                payment_hash: val.payment_hash,
                description_hash: val.description_hash,
                payment_secret: val.payment_secret.to_hex(),
                min_final_cltv_expiry_delta: val.min_final_cltv_expiry_delta,
                expiry: Duration::from_secs(val.expiry),
                expires_at: Utc
                    .timestamp_opt((val.timestamp + val.expiry) as i64, 0)
                    .unwrap(),
            }),
            ..Default::default()
        }
    }
}

impl From<BreezPayment> for Payment {
    fn from(val: BreezPayment) -> Self {
        Payment {
            ledger: Ledger::Lightning,
            payment_hash: Some(val.id),
            payment_preimage: match val.details {
                PaymentDetails::Ln { data } => Some(data.payment_preimage),
                _ => None,
            },
            error: val.error,
            amount_msat: val.amount_msat,
            fee_msat: Some(val.fee_msat),
            payment_time: Some(Utc.timestamp_opt(val.payment_time, 0).unwrap()),
            description: val.description,
            metadata: val.metadata,
            ..Default::default()
        }
    }
}

impl From<BreezNetwork> for Currency {
    fn from(val: BreezNetwork) -> Self {
        match val {
            BreezNetwork::Bitcoin => Currency::Bitcoin,
            BreezNetwork::Regtest => Currency::Regtest,
            BreezNetwork::Signet => Currency::Signet,
            BreezNetwork::Testnet => Currency::BitcoinTestnet,
        }
    }
}

impl From<BreezPayment> for LnInvoicePaidEvent {
    fn from(val: BreezPayment) -> Self {
        LnInvoicePaidEvent {
            payment_hash: val.id,
            amount_msat: val.amount_msat,
            fee_msat: val.fee_msat,
            payment_time: Utc.timestamp_opt(val.payment_time, 0).unwrap(),
        }
    }
}

impl From<BreezPayment> for LnPaySuccessEvent {
    fn from(val: BreezPayment) -> Self {
        LnPaySuccessEvent {
            amount_msat: val.amount_msat,
            fees_msat: val.fee_msat,
            payment_hash: val.id,
            payment_preimage: match val.details {
                PaymentDetails::Ln { data } => data.payment_preimage,
                _ => String::new(),
            },
            payment_time: Utc.timestamp_opt(val.payment_time, 0).unwrap(),
        }
    }
}

impl From<PaymentFailedData> for LnPayFailureEvent {
    fn from(val: PaymentFailedData) -> Self {
        LnPayFailureEvent {
            reason: val.error,
            payment_hash: val.invoice.unwrap().payment_hash,
        }
    }
}
