use chrono::{TimeZone, Utc};
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::{application::entities::Ledger, domains::payments::entities::Payment};

use super::cln::{pay_response::PayStatus, PayResponse};

impl From<PayResponse> for Payment {
    fn from(val: PayResponse) -> Self {
        let error = match val.status() {
            PayStatus::Complete => None,
            _ => Some(format!(
                "Unexpected error. Payment returned successfully but with status {}",
                val.status().as_str_name()
            )),
        };

        let seconds = val.created_at as i64;
        let nanoseconds = ((val.created_at - seconds as f64) * 1e9) as u32;

        Payment {
            ledger: Ledger::Lightning,
            payment_hash: Some(val.payment_hash.to_hex()),
            payment_preimage: Some(val.payment_preimage.to_hex()),
            amount_msat: val.amount_sent_msat.clone().unwrap().msat,
            fee_msat: Some(val.amount_sent_msat.unwrap().msat - val.amount_msat.unwrap().msat),
            payment_time: Some(Utc.timestamp_opt(seconds, nanoseconds).unwrap()),
            error,
            ..Default::default()
        }
    }
}
