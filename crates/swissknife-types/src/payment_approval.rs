use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
    Consumed,
    Expired,
}

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Reject,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DecidePaymentApproval {
    pub decision: ApprovalDecision,
}

/// Immutable payment terms shown to the account operator. Amounts are exact millisatoshi strings.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct PaymentApproval {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub agent_key_id: Uuid,
    pub idempotency_key: Uuid,
    pub requested_input: String,
    pub ledger: crate::Ledger,
    /// Lightning payment hash or the Bitcoin address to receive the funds.
    pub destination: String,
    pub amount_msat: String,
    pub maximum_fee_msat: String,
    pub state: ApprovalState,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by: Option<Uuid>,
}
