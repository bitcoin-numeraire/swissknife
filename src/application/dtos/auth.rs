use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Sign In Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignInRequest {
    /// User password
    pub password: String,
}

/// Sign In Response
#[derive(Debug, Serialize, ToSchema)]
pub struct SignInResponse {
    /// JWT token
    #[schema(example = "eyJ0eXAiOiJKV1QiLCJhbGciOiJ...")]
    pub token: String,
}
