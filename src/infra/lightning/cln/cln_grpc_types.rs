use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::{application::entities::Ledger, domains::payments::entities::Payment};

use super::cln::{pay_response::PayStatus, PayResponse};

impl Into<Payment> for PayResponse {
    fn into(self) -> Payment {
        let error = match self.status() {
            PayStatus::Complete => None,
            _ => Some(format!(
                "Unexpected error. Payment returned successfully but with status {}",
                self.status().as_str_name()
            )),
        };

        let seconds = self.created_at as i64;
        let nanoseconds = ((self.created_at - seconds as f64) * 1e9) as u32;

        Payment {
            ledger: Ledger::LIGHTNING,
            payment_hash: Some(self.payment_hash.to_hex()),
            payment_preimage: Some(self.payment_preimage.to_hex()),
            amount_msat: self.amount_sent_msat.clone().unwrap().msat,
            fee_msat: Some(self.amount_sent_msat.unwrap().msat - self.amount_msat.unwrap().msat),
            payment_time: Some(Utc.timestamp_opt(seconds, nanoseconds).unwrap()),
            error,
            ..Default::default()
        }
    }
}
