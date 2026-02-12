use std::time::Duration;

use breez_sdk_liquid::prelude::{
    LNInvoice, Network as BreezNetwork, Payment as BreezPayment, PaymentDetails, PaymentState, PaymentType,
};
use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::{
    application::entities::{Currency, Ledger},
    domains::{
        event::{LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent},
        invoice::{Invoice, InvoiceStatus, LnInvoice},
        payment::{LnPayment, Payment, PaymentStatus},
    },
};

fn timestamp_to_utc(timestamp: u32) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(timestamp as i64, 0).single().unwrap_or_else(Utc::now)
}

fn get_description(details: &PaymentDetails) -> Option<String> {
    match details {
        PaymentDetails::Lightning { description, .. }
        | PaymentDetails::Liquid { description, .. }
        | PaymentDetails::Bitcoin { description, .. } => Some(description.clone()),
    }
}

fn get_payment_hash(details: &PaymentDetails) -> Option<String> {
    match details {
        PaymentDetails::Lightning { payment_hash, .. } => payment_hash.clone(),
        _ => None,
    }
}

fn get_payment_preimage(details: &PaymentDetails) -> Option<String> {
    match details {
        PaymentDetails::Lightning { preimage, .. } => preimage.clone(),
        _ => None,
    }
}

fn get_lnurl_metadata(details: &PaymentDetails) -> Option<String> {
    match details {
        PaymentDetails::Lightning { lnurl_info, .. } => {
            lnurl_info.as_ref().and_then(|info| info.lnurl_pay_metadata.clone())
        }
        _ => None,
    }
}

fn get_ln_address(details: &PaymentDetails) -> Option<String> {
    match details {
        PaymentDetails::Lightning { lnurl_info, .. } => lnurl_info.as_ref().and_then(|info| info.ln_address.clone()),
        _ => None,
    }
}

impl From<LNInvoice> for Invoice {
    fn from(val: LNInvoice) -> Self {
        Invoice {
            ledger: Ledger::Lightning,
            description: val.description,
            amount_msat: val.amount_msat,
            timestamp: Utc.timestamp_opt(val.timestamp as i64, 0).unwrap(),
            ln_invoice: Some(LnInvoice {
                bolt11: val.bolt11,
                payee_pubkey: val.payee_pubkey,
                payment_hash: val.payment_hash,
                description_hash: val.description_hash,
                payment_secret: val.payment_secret.to_hex(),
                min_final_cltv_expiry_delta: val.min_final_cltv_expiry_delta,
                expiry: Duration::from_secs(val.expiry),
                expires_at: Utc.timestamp_opt((val.timestamp + val.expiry) as i64, 0).unwrap(),
            }),
            ..Default::default()
        }
    }
}

impl From<BreezPayment> for Payment {
    fn from(val: BreezPayment) -> Self {
        let details = val.details.clone();
        let payment_hash = get_payment_hash(&details);

        Payment {
            ledger: Ledger::Lightning,
            error: match val.status {
                PaymentState::Failed | PaymentState::TimedOut => Some("Payment failed".to_string()),
                _ => None,
            },
            amount_msat: val.amount_sat.saturating_mul(1000),
            fee_msat: Some(val.fees_sat.saturating_mul(1000)),
            payment_time: Some(timestamp_to_utc(val.timestamp)),
            status: val.status.into(),
            description: get_description(&details),
            lightning: payment_hash.clone().map(|hash| LnPayment {
                ln_address: get_ln_address(&details),
                payment_hash: hash,
                payment_preimage: get_payment_preimage(&details),
                metadata: get_lnurl_metadata(&details),
                success_action: None,
            }),
            ..Default::default()
        }
    }
}

impl From<BreezPayment> for Invoice {
    fn from(val: BreezPayment) -> Self {
        let details = val.details.clone();
        Invoice {
            ledger: Ledger::Lightning,
            description: get_description(&details),
            amount_msat: Some(val.amount_sat.saturating_mul(1000)),
            fee_msat: Some(val.fees_sat.saturating_mul(1000)),
            payment_time: Some(timestamp_to_utc(val.timestamp)),
            status: val.status.into(),
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

impl From<PaymentState> for PaymentStatus {
    fn from(val: PaymentState) -> Self {
        match val {
            PaymentState::Complete => PaymentStatus::Settled,
            PaymentState::Failed | PaymentState::TimedOut => PaymentStatus::Failed,
            PaymentState::Created
            | PaymentState::Pending
            | PaymentState::Refundable
            | PaymentState::RefundPending
            | PaymentState::WaitingFeeAcceptance => PaymentStatus::Pending,
        }
    }
}

impl From<PaymentState> for InvoiceStatus {
    fn from(val: PaymentState) -> Self {
        match val {
            PaymentState::Complete => InvoiceStatus::Settled,
            PaymentState::Failed | PaymentState::TimedOut => InvoiceStatus::Expired,
            PaymentState::Created
            | PaymentState::Pending
            | PaymentState::Refundable
            | PaymentState::RefundPending
            | PaymentState::WaitingFeeAcceptance => InvoiceStatus::Pending,
        }
    }
}

impl From<BreezPayment> for LnInvoicePaidEvent {
    fn from(val: BreezPayment) -> Self {
        let details = val.details.clone();
        LnInvoicePaidEvent {
            payment_hash: get_payment_hash(&details).unwrap_or_default(),
            amount_received_msat: val.amount_sat.saturating_mul(1000),
            fee_msat: val.fees_sat.saturating_mul(1000),
            payment_time: timestamp_to_utc(val.timestamp),
        }
    }
}

impl From<BreezPayment> for LnPaySuccessEvent {
    fn from(val: BreezPayment) -> Self {
        let details = val.details.clone();
        LnPaySuccessEvent {
            amount_msat: val.amount_sat.saturating_mul(1000),
            fees_msat: val.fees_sat.saturating_mul(1000),
            payment_hash: get_payment_hash(&details).unwrap_or_default(),
            payment_preimage: get_payment_preimage(&details).unwrap_or_default(),
            payment_time: timestamp_to_utc(val.timestamp),
        }
    }
}

impl From<BreezPayment> for LnPayFailureEvent {
    fn from(val: BreezPayment) -> Self {
        let details = val.details.clone();
        LnPayFailureEvent {
            reason: format!("Payment failed with status: {:?}", val.status),
            payment_hash: get_payment_hash(&details).unwrap_or_default(),
        }
    }
}

impl From<PaymentType> for Ledger {
    fn from(_val: PaymentType) -> Self {
        Ledger::Lightning
    }
}
