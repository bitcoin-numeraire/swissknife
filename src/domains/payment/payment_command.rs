use bitcoin::hashes::{sha256, Hash};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Payment;

#[derive(Clone, Debug)]
pub struct SubmissionKey {
    pub wallet_id: Uuid,
    pub key: Uuid,
    pub fingerprint: String,
}

impl SubmissionKey {
    pub fn new(wallet_id: Uuid, request: &swissknife_types::SubmitPaymentRequest) -> Self {
        // Fingerprint the original intent. LNURL resolution can return a new invoice on each call.
        let bytes = serde_json::to_vec(&(
            &request.input,
            request.amount_msat,
            &request.comment,
            request.maximum_fee_msat,
        ))
        .expect("strings and integers serialize");
        Self {
            wallet_id,
            key: request.idempotency_key,
            fingerprint: sha256::Hash::hash(&bytes).to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "rail", rename_all = "snake_case")]
pub enum PaymentInstructions {
    Lightning {
        bolt11: String,
        variable_amount_msat: Option<u64>,
        maximum_fee_msat: u64,
    },
}

#[derive(Clone, Debug)]
pub struct SubmittedPayment {
    pub payment: Payment,
    pub dispatched: bool,
}

#[derive(Clone, Debug)]
pub struct ClaimedPayment {
    pub payment: Payment,
    pub instructions: PaymentInstructions,
}

impl SubmittedPayment {
    pub fn response(self, base: &str) -> swissknife_types::PaymentSubmission {
        use swissknife_types::{PaymentExecutionState as State, PaymentStatus};
        let state = match self.payment.status {
            PaymentStatus::Settled => State::Settled,
            PaymentStatus::Failed => State::Failed,
            PaymentStatus::Pending if self.dispatched => State::Processing,
            PaymentStatus::Pending => State::Accepted,
        };
        let status_url = format!("{base}/{}", self.payment.id);
        swissknife_types::PaymentSubmission {
            payment: self.payment,
            state,
            status_url,
        }
    }
}
