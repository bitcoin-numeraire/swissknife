//! CLN's listpays accounts for active xpay retries; sendpay notifications do not.
//! A "complete" summary can still include pending MPP parts. Wait for all parts
//! to resolve and total only successful parts before committing the actual debit.
use bitcoin::hashes::{sha256, Hash};
use chrono::{TimeZone, Utc};
use serde::Deserialize;

use crate::{
    application::{composition::Ledger, errors::LightningError},
    domains::payment::{LnPayment, Payment, PaymentStatus},
};

#[derive(Debug, Deserialize)]
pub(crate) struct ClnPaymentPart {
    pub payment_hash: String,
    pub status: String,
    pub amount_msat: Option<u64>,
    pub amount_sent_msat: Option<u64>,
    pub payment_preimage: Option<String>,
    pub completed_at: Option<u64>,
}

pub(super) fn payment_state(payment_hash: String, status: &str) -> Result<Payment, LightningError> {
    let (status, error) = match status {
        "pending" | "complete" => (PaymentStatus::Pending, None),
        "failed" => (PaymentStatus::Failed, Some("Payment failed".to_string())),
        other => return Err(invalid(format!("Unknown aggregate payment status: {other}"))),
    };
    Ok(Payment {
        ledger: Ledger::Lightning,
        status,
        error,
        lightning: Some(LnPayment {
            payment_hash,
            ..Default::default()
        }),
        ..Default::default()
    })
}

fn invalid(reason: impl Into<String>) -> LightningError {
    LightningError::PaymentByHash(reason.into())
}

pub(super) fn completed_payment(payment_hash: String, parts: Vec<ClnPaymentPart>) -> Result<Payment, LightningError> {
    if parts.is_empty() || parts.iter().any(|part| part.payment_hash != payment_hash) {
        return Err(invalid("Missing or mismatched payment parts"));
    }
    let mut payment = payment_state(payment_hash.clone(), "pending")?;
    if parts.iter().any(|part| part.status == "pending") {
        return Ok(payment);
    }
    let mut amount = 0_u64;
    let mut sent = 0_u64;
    let mut completed_at = 0;
    let mut preimage = None;
    for part in parts {
        match part.status.as_str() {
            "failed" => continue,
            "complete" => {}
            _ => return Err(invalid("Unknown payment part status")),
        }
        let delivered = part.amount_msat.ok_or_else(|| invalid("Missing delivered amount"))?;
        let spent = part.amount_sent_msat.ok_or_else(|| invalid("Missing sent amount"))?;
        if spent < delivered {
            return Err(invalid("Sent amount is below delivered amount"));
        }
        amount = amount
            .checked_add(delivered)
            .ok_or_else(|| invalid("Payment amount overflow"))?;
        sent = sent
            .checked_add(spent)
            .ok_or_else(|| invalid("Payment amount overflow"))?;
        completed_at = completed_at.max(part.completed_at.ok_or_else(|| invalid("Missing completion time"))?);
        let proof = part
            .payment_preimage
            .ok_or_else(|| invalid("Missing payment preimage"))?;
        let bytes = hex::decode(&proof).map_err(|_| invalid("Invalid payment preimage"))?;
        if bytes.len() != 32 || sha256::Hash::hash(&bytes).to_string() != payment_hash {
            return Err(invalid("Payment preimage does not match hash"));
        }
        preimage = Some(proof);
    }
    if preimage.is_none() {
        return Err(invalid("No successful parts for completed payment"));
    }
    payment.status = PaymentStatus::Settled;
    payment.amount_msat = amount;
    payment.fee_msat = Some(sent - amount);
    payment.payment_time = Some(
        Utc.timestamp_opt(
            i64::try_from(completed_at).map_err(|_| invalid("Invalid completion time"))?,
            0,
        )
        .single()
        .ok_or_else(|| invalid("Invalid completion time"))?,
    );
    payment.lightning = Some(LnPayment {
        payment_hash,
        payment_preimage: preimage,
        ..Default::default()
    });
    Ok(payment)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash() -> String {
        sha256::Hash::hash(&[7; 32]).to_string()
    }
    fn part(status: &str, amount: u64, sent: u64) -> ClnPaymentPart {
        ClnPaymentPart {
            payment_hash: hash(),
            status: status.to_string(),
            amount_msat: Some(amount),
            amount_sent_msat: Some(sent),
            payment_preimage: Some(hex::encode([7; 32])),
            completed_at: Some(1_700_000_000),
        }
    }

    mod completed_payment {
        use super::*;
        #[test]
        fn retains_pending_when_other_parts_are_in_flight() {
            let result = completed_payment(
                hash(),
                vec![part("complete", 60_000, 61_000), part("pending", 40_000, 42_000)],
            )
            .unwrap();
            assert_eq!(result.status, PaymentStatus::Pending);
        }
        #[test]
        fn totals_successful_parts_without_charging_failed_attempts() {
            let result = completed_payment(
                hash(),
                vec![
                    part("failed", 100_000, 109_000),
                    part("complete", 60_000, 61_000),
                    part("complete", 40_000, 42_000),
                ],
            )
            .unwrap();
            assert_eq!(result.status, PaymentStatus::Settled);
            assert_eq!(result.amount_msat, 100_000);
            assert_eq!(result.fee_msat, Some(3_000));
        }
        #[test]
        fn rejects_unverifiable_or_incomplete_successes() {
            let mut missing = part("complete", 1, 1);
            missing.payment_preimage = None;
            let mut wrong_hash = part("complete", 1, 1);
            wrong_hash.payment_hash = "00".repeat(32);
            let mut wrong_proof = part("complete", 1, 1);
            wrong_proof.payment_preimage = Some("00".repeat(32));
            let mut missing_amount = part("complete", 1, 1);
            missing_amount.amount_msat = None;
            for parts in [
                vec![],
                vec![missing],
                vec![wrong_hash],
                vec![wrong_proof],
                vec![missing_amount],
                vec![part("complete", 2, 1)],
                vec![part("failed", 1, 1)],
                vec![part("unknown", 1, 1)],
            ] {
                assert!(completed_payment(hash(), parts).is_err());
            }
        }
    }
}
