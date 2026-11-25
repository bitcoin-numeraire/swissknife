use bitcoin::hashes::{sha256, Hash};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Payment;

#[derive(Clone, Debug)]
pub struct SubmissionKey {
    pub wallet_id: Uuid,
    pub key: Uuid,
    pub fingerprint: String,
    pub agent_key_id: Option<Uuid>,
}

impl SubmissionKey {
    pub fn new(wallet_id: Uuid, request: &swissknife_types::SubmitPaymentRequest, agent_key_id: Option<Uuid>) -> Self {
        // Fingerprint the original intent. LNURL resolution can return a new invoice on each call.
        let intent = (
            &request.input,
            request.amount_msat,
            &request.comment,
            request.maximum_fee_msat,
        );
        // Preserve fingerprints of requests admitted before BIP353.
        let bytes = match &request.expected_offer_id {
            Some(id) => serde_json::to_vec(&(intent, id)),
            None => serde_json::to_vec(&intent),
        }
        .expect("strings and integers serialize");
        Self {
            wallet_id,
            key: request.idempotency_key,
            agent_key_id,
            fingerprint: sha256::Hash::hash(&bytes).to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "rail", rename_all = "snake_case")]
pub enum PaymentInstructions {
    Onchain {
        backend: String,
        address: String,
        maximum_fee_msat: u64,
    },
    Lightning {
        /// Earlier installations stored the wire invoice under this field name.
        #[serde(rename = "bolt11")]
        invoice: String,
        variable_amount_msat: Option<u64>,
        maximum_fee_msat: u64,
    },
}

#[derive(Clone, Debug)]
pub struct SubmittedPayment {
    pub payment: Payment,
    pub dispatched: bool,
    pub withdrawal: Option<super::StoredWithdrawal>,
}

#[derive(Clone, Debug)]
pub struct ClaimedPayment {
    pub payment: Payment,
    pub instructions: PaymentInstructions,
}

impl SubmittedPayment {
    pub fn check_original_backend(mut self, backend: &str) -> Self {
        if let Some(withdrawal) = &mut self.withdrawal {
            withdrawal.details.awaiting_original_provider =
                self.payment.status == super::PaymentStatus::Pending && withdrawal.backend != backend;
        }
        self
    }
    pub fn response(self, base: &str) -> swissknife_types::PaymentSubmission {
        use swissknife_types::{PaymentExecutionState as State, PaymentStatus};
        let (execution, withdrawal) = self.withdrawal.map(|w| (w.state, w.details)).unzip();
        let state = match (&self.payment.status, execution) {
            (PaymentStatus::Settled, _) => State::Settled,
            (_, Some(state)) => state,
            (PaymentStatus::Failed, _) => State::Failed,
            (PaymentStatus::Pending, _) if self.dispatched => State::Processing,
            (PaymentStatus::Pending, _) => State::Accepted,
        };
        let status_url = format!("{base}/{}", self.payment.id);
        swissknife_types::PaymentSubmission {
            payment: self.payment,
            state,
            status_url,
            withdrawal,
        }
    }
}
