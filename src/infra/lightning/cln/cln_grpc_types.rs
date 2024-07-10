use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde_bolt::bitcoin::hashes::hex::ToHex;
use std::str::FromStr;

use crate::{
    application::entities::Ledger,
    domains::{
        invoices::entities::{Invoice, InvoiceStatus},
        lightning::entities::LnInvoicePaidEvent,
        payments::entities::Payment,
    },
};

use super::cln::{
    listinvoices_invoices::ListinvoicesInvoicesStatus, pay_response::PayStatus,
    ListinvoicesInvoices, PayResponse, WaitanyinvoiceResponse,
};

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

impl From<ListinvoicesInvoices> for Invoice {
    fn from(val: ListinvoicesInvoices) -> Self {
        let bolt11_str = val.bolt11.clone().unwrap();
        let bolt11 = Bolt11Invoice::from_str(&bolt11_str).unwrap();
        let mut invoice: Invoice = bolt11.into();

        match val.status() {
            ListinvoicesInvoicesStatus::Paid => {
                invoice.status = InvoiceStatus::Settled;
                invoice.payment_time = Some(Utc.timestamp_opt(val.paid_at() as i64, 0).unwrap());
                invoice.amount_msat = Some(val.amount_received_msat.unwrap().msat);
            }
            ListinvoicesInvoicesStatus::Unpaid => {
                invoice.status = InvoiceStatus::Pending;
            }
            ListinvoicesInvoicesStatus::Expired => {
                invoice.status = InvoiceStatus::Expired;
            }
        };

        invoice
    }
}

impl From<WaitanyinvoiceResponse> for LnInvoicePaidEvent {
    fn from(val: WaitanyinvoiceResponse) -> Self {
        LnInvoicePaidEvent {
            id: None,
            payment_hash: Some(val.payment_hash.to_hex()),
            amount_received_msat: val.amount_received_msat.as_ref().unwrap().msat,
            fee_msat: 0,
            payment_time: Utc.timestamp_opt(val.paid_at() as i64, 0).unwrap(),
        }
    }
}
