use breez_sdk_spark::{
    BitcoinNetwork as SparkBitcoinNetwork, Network as SparkNetwork, Payment as SparkPayment,
    PaymentDetails as SparkPaymentDetails, PaymentMethod as SparkPaymentMethod, PaymentStatus as SparkPaymentStatus,
};
use chrono::{TimeZone, Utc};

use crate::{
    application::entities::{Currency, Ledger},
    domains::{
        event::{
            LnInvoicePaidEvent, LnPayFailureEvent, LnPaySuccessEvent, OnchainDepositEvent, OnchainWithdrawalEvent,
        },
        invoice::{Invoice, InvoiceStatus},
        payment::{LnPayment, Payment, PaymentStatus},
    },
};

fn timestamp_to_utc(timestamp: u64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(timestamp as i64, 0).single().unwrap_or_else(Utc::now)
}

fn get_description(details: &Option<SparkPaymentDetails>) -> Option<String> {
    match details {
        Some(SparkPaymentDetails::Lightning { description, .. }) => description.clone(),
        Some(SparkPaymentDetails::Spark { invoice_details, .. }) => {
            invoice_details.as_ref().and_then(|d| d.description.clone())
        }
        _ => None,
    }
}

fn get_payment_hash(details: &Option<SparkPaymentDetails>) -> Option<String> {
    match details {
        Some(SparkPaymentDetails::Lightning { htlc_details, .. }) => Some(htlc_details.payment_hash.clone()),
        Some(SparkPaymentDetails::Spark {
            htlc_details: Some(htlc),
            ..
        }) => Some(htlc.payment_hash.clone()),
        _ => None,
    }
}

fn get_payment_preimage(details: &Option<SparkPaymentDetails>) -> Option<String> {
    match details {
        Some(SparkPaymentDetails::Lightning { htlc_details, .. }) => htlc_details.preimage.clone(),
        Some(SparkPaymentDetails::Spark {
            htlc_details: Some(htlc),
            ..
        }) => htlc.preimage.clone(),
        _ => None,
    }
}

fn get_ln_address(details: &Option<SparkPaymentDetails>) -> Option<String> {
    match details {
        Some(SparkPaymentDetails::Lightning { lnurl_pay_info, .. }) => {
            lnurl_pay_info.as_ref().and_then(|info| info.ln_address.clone())
        }
        _ => None,
    }
}

fn get_lnurl_metadata(details: &Option<SparkPaymentDetails>) -> Option<String> {
    match details {
        Some(SparkPaymentDetails::Lightning {
            lnurl_receive_metadata, ..
        }) => lnurl_receive_metadata
            .as_ref()
            .and_then(|m| serde_json::to_string(m).ok()),
        _ => None,
    }
}

impl From<SparkPayment> for Payment {
    fn from(val: SparkPayment) -> Self {
        let payment_hash = get_payment_hash(&val.details);
        let amount_sat = val.amount as u64;
        let fees_sat = val.fees as u64;

        Payment {
            ledger: match val.method {
                SparkPaymentMethod::Deposit | SparkPaymentMethod::Withdraw => Ledger::Onchain,
                _ => Ledger::Lightning,
            },
            error: match val.status {
                SparkPaymentStatus::Failed => Some("Payment failed".to_string()),
                _ => None,
            },
            amount_msat: amount_sat.saturating_mul(1000),
            fee_msat: Some(fees_sat.saturating_mul(1000)),
            payment_time: Some(timestamp_to_utc(val.timestamp)),
            status: val.status.into(),
            description: get_description(&val.details),
            lightning: payment_hash.map(|hash| LnPayment {
                ln_address: get_ln_address(&val.details),
                payment_hash: hash,
                payment_preimage: get_payment_preimage(&val.details),
                metadata: get_lnurl_metadata(&val.details),
                success_action: None,
            }),
            ..Default::default()
        }
    }
}

impl From<SparkPayment> for Invoice {
    fn from(val: SparkPayment) -> Self {
        let amount_sat = val.amount as u64;
        let fees_sat = val.fees as u64;

        Invoice {
            ledger: Ledger::Lightning,
            description: get_description(&val.details),
            amount_msat: Some(amount_sat.saturating_mul(1000)),
            fee_msat: Some(fees_sat.saturating_mul(1000)),
            payment_time: Some(timestamp_to_utc(val.timestamp)),
            status: val.status.into(),
            ..Default::default()
        }
    }
}

impl From<SparkNetwork> for Currency {
    fn from(val: SparkNetwork) -> Self {
        match val {
            SparkNetwork::Mainnet => Currency::Bitcoin,
            SparkNetwork::Regtest => Currency::Regtest,
        }
    }
}

impl From<SparkBitcoinNetwork> for Currency {
    fn from(val: SparkBitcoinNetwork) -> Self {
        match val {
            SparkBitcoinNetwork::Bitcoin => Currency::Bitcoin,
            SparkBitcoinNetwork::Testnet3 | SparkBitcoinNetwork::Testnet4 => Currency::BitcoinTestnet,
            SparkBitcoinNetwork::Signet => Currency::Signet,
            SparkBitcoinNetwork::Regtest => Currency::Regtest,
        }
    }
}

impl From<SparkPaymentStatus> for PaymentStatus {
    fn from(val: SparkPaymentStatus) -> Self {
        match val {
            SparkPaymentStatus::Completed => PaymentStatus::Settled,
            SparkPaymentStatus::Failed => PaymentStatus::Failed,
            SparkPaymentStatus::Pending => PaymentStatus::Pending,
        }
    }
}

impl From<SparkPaymentStatus> for InvoiceStatus {
    fn from(val: SparkPaymentStatus) -> Self {
        match val {
            SparkPaymentStatus::Completed => InvoiceStatus::Settled,
            SparkPaymentStatus::Failed => InvoiceStatus::Expired,
            SparkPaymentStatus::Pending => InvoiceStatus::Pending,
        }
    }
}

impl From<SparkPayment> for LnInvoicePaidEvent {
    fn from(val: SparkPayment) -> Self {
        let payment_hash = get_payment_hash(&val.details).unwrap_or_default();
        let amount_sat = val.amount as u64;
        let fees_sat = val.fees as u64;

        LnInvoicePaidEvent {
            payment_hash,
            amount_received_msat: amount_sat.saturating_mul(1000),
            fee_msat: fees_sat.saturating_mul(1000),
            payment_time: timestamp_to_utc(val.timestamp),
        }
    }
}

impl From<SparkPayment> for LnPaySuccessEvent {
    fn from(val: SparkPayment) -> Self {
        let payment_hash = get_payment_hash(&val.details).unwrap_or_default();
        let preimage = get_payment_preimage(&val.details).unwrap_or_default();
        let amount_sat = val.amount as u64;
        let fees_sat = val.fees as u64;

        LnPaySuccessEvent {
            amount_msat: amount_sat.saturating_mul(1000),
            fees_msat: fees_sat.saturating_mul(1000),
            payment_hash,
            payment_preimage: preimage,
            payment_time: timestamp_to_utc(val.timestamp),
        }
    }
}

impl From<SparkPayment> for LnPayFailureEvent {
    fn from(val: SparkPayment) -> Self {
        let payment_hash = get_payment_hash(&val.details).unwrap_or_default();

        LnPayFailureEvent {
            reason: format!("Payment failed with status: {:?}", val.status),
            payment_hash,
        }
    }
}

// -- Bitcoin on-chain event helpers --

/// Returns true if the payment is an on-chain deposit or withdrawal.
pub fn is_onchain_payment(method: &SparkPaymentMethod) -> bool {
    matches!(method, SparkPaymentMethod::Deposit | SparkPaymentMethod::Withdraw)
}

/// Converts a Spark Withdraw payment into an OnchainWithdrawalEvent.
pub fn to_withdrawal_event(payment: &SparkPayment) -> Option<OnchainWithdrawalEvent> {
    let txid = match &payment.details {
        Some(SparkPaymentDetails::Withdraw { tx_id }) => tx_id.clone(),
        _ => return None,
    };

    let block_height = match payment.status {
        SparkPaymentStatus::Completed => Some(1), // Sentinel: Spark doesn't expose block height
        _ => None,
    };

    Some(OnchainWithdrawalEvent { txid, block_height })
}

/// Converts a Spark Deposit payment into an OnchainDepositEvent.
/// NOTE: Spark's Deposit details only contain tx_id, not the bitcoin address.
pub fn to_deposit_event(payment: &SparkPayment) -> Option<OnchainDepositEvent> {
    let txid = match &payment.details {
        Some(SparkPaymentDetails::Deposit { tx_id }) => tx_id.clone(),
        _ => return None,
    };

    let block_height = match payment.status {
        SparkPaymentStatus::Completed => Some(1),
        _ => None,
    };

    Some(OnchainDepositEvent {
        txid,
        output_index: 0,
        address: String::new(), // Spark Deposit doesn't provide the address in PaymentDetails
        amount_sat: payment.amount as u64,
        block_height,
    })
}

/// Converts a failed Bitcoin payment into an LnPayFailureEvent (using tx_id as identifier).
pub fn to_bitcoin_failure_event(payment: &SparkPayment) -> Option<LnPayFailureEvent> {
    let txid = match &payment.details {
        Some(SparkPaymentDetails::Deposit { tx_id }) | Some(SparkPaymentDetails::Withdraw { tx_id }) => tx_id.clone(),
        _ => return None,
    };

    Some(LnPayFailureEvent {
        reason: format!("Bitcoin operation failed with status: {:?}", payment.status),
        payment_hash: txid,
    })
}
