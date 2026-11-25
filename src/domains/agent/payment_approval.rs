use serde::{Deserialize, Serialize};
use swissknife_types::{Ledger, LnUrlPaySuccessAction};
use uuid::Uuid;

/// Terms resolved before review. The credential retries the original request, while the
/// wallet revalidates the stored invoice or Bitcoin address and the current spending rules.
#[derive(Clone, Serialize, Deserialize)]
pub struct ReviewedPayment {
    pub requested_input: String,
    pub input: String,
    pub destination: String,
    pub ledger: Ledger,
    pub withdrawal_backend: Option<String>,
    pub recipient_wallet: Option<Uuid>,
    pub amount_msat: u64,
    pub maximum_fee_msat: u64,
    pub description: Option<String>,
    pub ln_address: Option<String>,
    pub success_action: Option<LnUrlPaySuccessAction>,
}

impl ReviewedPayment {
    pub fn allows(&self, proposed: &Self) -> bool {
        self.ledger == proposed.ledger
            && self.input == proposed.input
            && self.destination == proposed.destination
            && self.withdrawal_backend == proposed.withdrawal_backend
            && self.recipient_wallet == proposed.recipient_wallet
            && self.amount_msat == proposed.amount_msat
            && proposed.maximum_fee_msat <= self.maximum_fee_msat
    }
}
