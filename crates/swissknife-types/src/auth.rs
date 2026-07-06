use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

/// Authentication provider namespace.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, EnumString, Display, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AuthProvider {
    #[default]
    Jwt,
    OAuth2,
}

/// Sign Up Request
#[derive(Debug, Deserialize, ToSchema, Serialize)]
pub struct SignUpRequest {
    /// User password
    #[schema(example = "password")]
    pub password: String,
}

/// Sign In Request
#[derive(Debug, Deserialize, ToSchema, Serialize)]
pub struct SignInRequest {
    /// User password
    #[schema(example = "password")]
    pub password: String,
}

/// Change Password Request
#[derive(Debug, Deserialize, ToSchema, Serialize)]
pub struct ChangePasswordRequest {
    /// Current user password
    #[schema(example = "old-password")]
    pub current_password: String,
    /// New user password
    #[schema(example = "new-password")]
    pub new_password: String,
}

/// Sign In Response
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SignInResponse {
    /// JWT token
    #[schema(example = "eyJ0eXAiOiJKV1QiLCJhbGciOiJ...")]
    pub token: String,
}
