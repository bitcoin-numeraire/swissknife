use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

/// Application error response
#[derive(Debug, Deserialize, Serialize, ToResponse, ToSchema)]
pub struct ErrorResponse {
    /// Error status
    #[schema(example = "401 Unauthorized")]
    pub status: String,

    /// Error reason
    #[schema(example = "error message")]
    pub reason: String,

    /// Optional machine-readable explanation for clients that can act on this error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<ErrorCode>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// No command was accepted. The destination belongs to this installation.
    InternalTransferRequired,
    /// The reviewed BIP353 offer changed before admission. Request a new preview.
    PaymentTargetChanged,
}
