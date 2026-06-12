use serde::Serialize;
use utoipa::ToSchema;

pub use swissknife_api_types::{SignInRequest, SignUpRequest};

/// Sign In Response
#[derive(Debug, Serialize, ToSchema)]
pub struct SignInResponse {
    /// JWT token
    #[schema(example = "eyJ0eXAiOiJKV1QiLCJhbGciOiJ...")]
    pub token: String,
}
