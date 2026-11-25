use serde::{Deserialize, Serialize};
use swissknife_types::{PaymentExecutionState, WithdrawalSubmission};
use uuid::Uuid;

use super::Payment;
use crate::domains::bitcoin::{BtcBroadcastTransaction, BtcPreparedTransaction};

/// Provider artifacts stay in the database. Only the public execution state is serialized to clients.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum WithdrawalProgress {
    Accepted,
    Funding {
        owner: Uuid,
        waiting: bool,
    },
    Planned {
        owner: Uuid,
        plan: BtcPreparedTransaction,
        waiting: bool,
    },
    Signed {
        transaction: BtcBroadcastTransaction,
        fee_msat: u64,
        publication: Publication,
    },
    Failed {
        reason: String,
    },
    Cancelled,
    Confirmed,
    Expired,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Publication {
    Ready,
    Unknown,
    Broadcast,
}

impl WithdrawalProgress {
    /// Once publication is possible, reconciliation can only replay exactly the same signed transaction.
    pub fn permits(&self, next: &Self) -> bool {
        match (self, next) {
            (
                Self::Signed {
                    transaction, fee_msat, ..
                },
                Self::Signed {
                    transaction: retry,
                    fee_msat: fee,
                    ..
                },
            ) => transaction == retry && fee_msat == fee,
            (
                Self::Planned { plan, .. },
                Self::Signed {
                    fee_msat,
                    publication: Publication::Ready,
                    ..
                },
            ) => plan.fee_sat.checked_mul(1000) == Some(*fee_msat),
            (Self::Accepted, Self::Funding { .. } | Self::Expired) => true,
            (
                Self::Funding { .. },
                Self::Funding { .. } | Self::Planned { .. } | Self::Failed { .. } | Self::Expired,
            ) => true,
            (Self::Planned { .. }, Self::Funding { .. } | Self::Expired) => true,
            _ => false,
        }
    }

    pub fn state(&self) -> PaymentExecutionState {
        use PaymentExecutionState as State;
        match self {
            Self::Accepted => State::Accepted,
            Self::Funding { waiting: true, .. } | Self::Planned { waiting: true, .. } => State::Waiting,
            Self::Funding { .. } | Self::Planned { .. } => State::Processing,
            Self::Signed { publication, .. } => match publication {
                Publication::Ready => State::Processing,
                Publication::Unknown => State::Reconciling,
                Publication::Broadcast => State::Broadcast,
            },
            Self::Failed { .. } => State::Failed,
            Self::Cancelled => State::Cancelled,
            Self::Confirmed => State::Settled,
            Self::Expired => State::Expired,
        }
    }

    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            Self::Failed { reason } => Some(reason),
            Self::Expired => Some("Withdrawal expired before broadcast"),
            Self::Cancelled => Some("Withdrawal cancelled before execution"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClaimedWithdrawal {
    pub payment: Payment,
    pub token: Uuid,
    pub original_backend: bool,
    pub details: WithdrawalSubmission,
    pub progress: WithdrawalProgress,
}

#[derive(Clone, Debug)]
pub struct StoredWithdrawal {
    pub backend: String,
    pub state: PaymentExecutionState,
    pub details: WithdrawalSubmission,
}
