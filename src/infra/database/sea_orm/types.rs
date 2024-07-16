use std::time::Duration;

use chrono::Utc;

use crate::domains::{
    invoice::{Invoice, InvoiceStatus, LnInvoice},
    ln_address::LnAddress,
    payment::Payment,
    wallet::{Contact, UserBalance},
};

use super::models::ln_address::Model as LnAddressModel;
use super::models::payment::Model as PaymentModel;
use super::models::{
    contact::ContactModel, invoice::Model as InvoiceModel, user_balance::UserBalanceModel,
};

const ASSERTION_MSG: &str = "should parse successfully by assertion";

impl From<InvoiceModel> for Invoice {
    fn from(model: InvoiceModel) -> Self {
        let status = match model.payment_time {
            Some(_) => InvoiceStatus::Settled,
            None => match model.expires_at {
                Some(expires_at) if Utc::now() > expires_at => InvoiceStatus::Expired,
                _ => InvoiceStatus::Pending,
            },
        };

        let ln_invoice = match model.ledger.as_str() {
            "Lightning" => Some(LnInvoice {
                payment_hash: model.payment_hash.expect(ASSERTION_MSG),
                bolt11: model.bolt11.expect(ASSERTION_MSG),
                description_hash: model.description_hash,
                payee_pubkey: model.payee_pubkey.expect(ASSERTION_MSG),
                min_final_cltv_expiry_delta: model.min_final_cltv_expiry_delta.expect(ASSERTION_MSG)
                    as u64,
                payment_secret: model.payment_secret.expect(ASSERTION_MSG),
                expiry: Duration::from_secs(model.expiry.expect(ASSERTION_MSG) as u64),
                expires_at: model.expires_at.expect(ASSERTION_MSG),
            }),
            _ => None,
        };

        Invoice {
            id: model.id,
            wallet_id: model.wallet_id,
            ln_address_id: model.ln_address_id,
            description: model.description,
            amount_msat: model.amount_msat.map(|v| v as u64),
            amount_received_msat: model.amount_received_msat.map(|v| v as u64),
            timestamp: model.timestamp.into(),
            ledger: model.ledger.parse().expect(ASSERTION_MSG),
            status,
            fee_msat: None,
            payment_time: model.payment_time.into(),
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
            ln_invoice,
        }
    }
}

impl From<PaymentModel> for Payment {
    fn from(model: PaymentModel) -> Self {
        Payment {
            id: model.id,
            wallet_id: model.wallet_id,
            ln_address: model.ln_address,
            payment_hash: model.payment_hash,
            payment_preimage: model.payment_preimage,
            error: model.error,
            amount_msat: model.amount_msat as u64,
            fee_msat: model.fee_msat.map(|v| v as u64),
            payment_time: model.payment_time,
            status: model.status.parse().expect(ASSERTION_MSG),
            ledger: model.ledger.parse().expect(ASSERTION_MSG),
            description: model.description,
            metadata: model.metadata,
            success_action: serde_json::from_value(model.success_action.unwrap_or_default()).ok(),
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl From<ContactModel> for Contact {
    fn from(model: ContactModel) -> Self {
        Contact {
            ln_address: model.ln_address,
            contact_since: model.contact_since,
        }
    }
}

impl From<LnAddressModel> for LnAddress {
    fn from(model: LnAddressModel) -> Self {
        LnAddress {
            id: model.id,
            user_id: model.user_id,
            username: model.username,
            active: model.active,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl From<UserBalanceModel> for UserBalance {
    fn from(model: UserBalanceModel) -> Self {
        UserBalance {
            received_msat: model.received_msat as u64,
            sent_msat: model.sent_msat as u64,
            fees_paid_msat: model.fees_paid_msat as u64,
            available_msat: model.received_msat - (model.sent_msat + model.fees_paid_msat),
        }
    }
}
