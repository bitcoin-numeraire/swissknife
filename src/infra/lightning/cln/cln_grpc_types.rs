use chrono::{TimeZone, Utc};
use lightning_invoice::Bolt11Invoice;
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use std::str::FromStr;

use crate::{
    application::composition::Ledger,
    domains::{
        bitcoin::BtcOutputStatus,
        event::LnInvoicePaidEvent,
        invoice::{Invoice, InvoiceStatus},
        payment::{LnPayment, Payment},
    },
    infra::lightning::cln::cln::listfunds_outputs::ListfundsOutputsStatus,
};

use super::cln::{
    listinvoices_invoices::ListinvoicesInvoicesStatus, ListinvoicesInvoices, WaitinvoiceResponse, XpayResponse,
};

impl From<XpayResponse> for Payment {
    fn from(val: XpayResponse) -> Self {
        // `xpay` returns no payment_hash; it is the SHA-256 of the preimage.
        let payment_hash = hex::encode(sha256::Hash::hash(&val.payment_preimage).to_byte_array());
        let amount_msat = val.amount_msat.map(|a| a.msat).unwrap_or_default();
        let amount_sent_msat = val.amount_sent_msat.map(|a| a.msat).unwrap_or(amount_msat);

        Payment {
            ledger: Ledger::Lightning,
            amount_msat,
            fee_msat: Some(amount_sent_msat.saturating_sub(amount_msat)),
            // A returned XpayResponse means the payment completed; xpay surfaces
            // failures as a gRPC error. No created_at is returned, so stamp now.
            payment_time: Some(Utc::now()),
            error: None,
            lightning: Some(LnPayment {
                payment_hash,
                payment_preimage: Some(hex::encode(val.payment_preimage)),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl From<ListinvoicesInvoices> for Invoice {
    fn from(val: ListinvoicesInvoices) -> Self {
        let bolt11_str = val.bolt11.clone().unwrap();
        let bolt11 = Bolt11Invoice::from_str(&bolt11_str).unwrap();
        let mut invoice: Invoice = crate::infra::lightning::types::invoice_from_bolt11(bolt11);

        match val.status() {
            ListinvoicesInvoicesStatus::Paid => {
                invoice.status = InvoiceStatus::Settled;
                invoice.payment_time = Some(Utc.timestamp_opt(val.paid_at() as i64, 0).unwrap());
                invoice.amount_received_msat = Some(val.amount_received_msat.unwrap().msat);
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

impl From<WaitinvoiceResponse> for LnInvoicePaidEvent {
    fn from(val: WaitinvoiceResponse) -> Self {
        LnInvoicePaidEvent {
            payment_hash: hex::encode(&val.payment_hash),
            amount_received_msat: val.amount_received_msat.as_ref().unwrap().msat,
            fee_msat: 0,
            payment_time: Utc.timestamp_opt(val.paid_at() as i64, 0).unwrap(),
        }
    }
}

impl From<ListfundsOutputsStatus> for BtcOutputStatus {
    fn from(val: ListfundsOutputsStatus) -> Self {
        match val {
            ListfundsOutputsStatus::Unconfirmed => BtcOutputStatus::Unconfirmed,
            ListfundsOutputsStatus::Confirmed => BtcOutputStatus::Confirmed,
            ListfundsOutputsStatus::Spent => BtcOutputStatus::Spent,
            ListfundsOutputsStatus::Immature => BtcOutputStatus::Immature,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::lightning::cln::cln::Amount;

    #[test]
    fn a_paid_invoice_preserves_requested_and_received_amounts_separately() {
        use bitcoin::{
            hashes::Hash,
            secp256k1::{Secp256k1, SecretKey},
        };
        use lightning_invoice::{Currency, InvoiceBuilder, PaymentSecret};

        let key = SecretKey::from_slice(&[1; 32]).unwrap();
        let bolt11 = InvoiceBuilder::new(Currency::Regtest)
            .description("invoice replay".to_string())
            .payment_hash(bitcoin::hashes::sha256::Hash::from_byte_array([42; 32]))
            .payment_secret(PaymentSecret([43; 32]))
            .duration_since_epoch(std::time::Duration::from_secs(1_791_000_000))
            .min_final_cltv_expiry_delta(18)
            .amount_milli_satoshis(1_000)
            .build_signed(|hash| Secp256k1::new().sign_ecdsa_recoverable(hash, &key))
            .unwrap();
        let response = ListinvoicesInvoices {
            bolt11: Some(bolt11.to_string()),
            status: ListinvoicesInvoicesStatus::Paid as i32,
            paid_at: Some(1_791_000_001),
            amount_received_msat: Some(Amount { msat: 1_100 }),
            ..Default::default()
        };

        let invoice: Invoice = response.into();

        assert_eq!(invoice.status, InvoiceStatus::Settled);
        assert_eq!(invoice.amount_msat, Some(1_000));
        assert_eq!(invoice.amount_received_msat, Some(1_100));
    }

    /// `amount_msat` must be the delivered amount and `fee_msat` the routing fee, so
    /// the settlement debit (`amount_msat + fee_msat`) equals `amount_sent_msat` once.
    /// Mirrors the REST converter test; together they pin the semantics the single-hop
    /// integration topology (zero routing fee) cannot exercise.
    #[test]
    fn xpay_response_separates_delivered_amount_from_routing_fee() {
        let resp = XpayResponse {
            payment_preimage: vec![1u8; 32],
            failed_parts: 0,
            successful_parts: 1,
            amount_msat: Some(Amount { msat: 100_000 }), // delivered to the recipient
            amount_sent_msat: Some(Amount { msat: 100_500 }), // delivered + 500 msat fee
        };

        let payment: Payment = resp.into();

        assert_eq!(payment.amount_msat, 100_000);
        assert_eq!(payment.fee_msat, Some(500));
        assert_eq!(payment.amount_msat + payment.fee_msat.unwrap(), 100_500);
    }
}
