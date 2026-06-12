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
}
