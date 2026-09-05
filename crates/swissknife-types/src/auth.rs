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
    /// Owner password: at least 15 Unicode characters and at most 1024 UTF-8 bytes.
    #[schema(example = "owner-specific passphrase")]
    pub password: String,
}

/// Sign In Request
#[derive(Debug, Deserialize, ToSchema, Serialize)]
pub struct SignInRequest {
    /// Local login username.
    pub username: String,
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
    /// New password: at least 15 Unicode characters and at most 1024 UTF-8 bytes.
    #[schema(example = "a different long passphrase")]
    pub new_password: String,
}

/// Sign In Response
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SignInResponse {
    /// JWT token
    #[schema(example = "eyJ0eXAiOiJKV1QiLCJhbGciOiJ...")]
    pub token: String,
}

/// Add a local login to an account. The response contains a one-time activation code.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct CreateLocalLoginRequest {
    pub username: String,
}

/// Enable or disable password login for this account.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateLocalLoginRequest {
    pub enabled: bool,
}

/// Non-secret local login information for account administrators.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct LocalLogin {
    pub username: String,
    pub enabled: bool,
    pub password_set: bool,
    pub reset_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// One-time activation or reset code. Shown only in this response.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct LocalLoginReset {
    pub code: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Redeem a local login activation/reset code and choose a password.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct ResetLocalPasswordRequest {
    pub code: String,
    /// At least 15 Unicode characters and at most 1024 UTF-8 bytes.
    #[schema(example = "a new personal passphrase")]
    pub new_password: String,
}
