use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

/// Authentication provider namespace.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, EnumString, Display, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AuthProvider {
    /// SwissKnife-managed username/password credentials and signed JWT sessions.
    #[default]
    Jwt,
    /// Access tokens issued by an external OAuth2/OpenID Connect provider.
    OAuth2,
}

/// Sign Up Request
#[derive(Debug, Deserialize, ToSchema, Serialize)]
pub struct SignUpRequest {
    /// Owner username, normalized to lowercase. Use 3 to 64 ASCII letters,
    /// digits, dots, underscores, or hyphens.
    #[schema(example = "admin")]
    pub username: String,
    /// Optional name shown in the dashboard. Blank names are treated as unset.
    #[schema(example = "Alice")]
    pub display_name: Option<String>,
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
    /// Unique username, trimmed and lowercased before storage. Use 3 to 64 ASCII
    /// letters, digits, dots, underscores, or hyphens.
    #[schema(example = "alice")]
    pub username: String,
}

/// Enable or disable password login for this account.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateLocalLoginRequest {
    /// Whether password sign-in is allowed. Changing this revokes local JWT
    /// sessions and pending reset codes; API keys retain their own permissions.
    pub enabled: bool,
}

/// Non-secret local login information for account administrators.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct LocalLogin {
    /// Normalized username for this account's password sign-in.
    #[schema(example = "alice")]
    pub username: String,
    /// Whether this login is enabled. A password must also be set to sign in.
    pub enabled: bool,
    /// Whether the account has a password; false while activation or reset is pending.
    pub password_set: bool,
    /// Expiration of the pending activation/reset code, or null if none was issued.
    /// An expiration in the past means the code can no longer be redeemed.
    pub reset_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// One-time activation or reset code. Shown only in this response.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct LocalLoginReset {
    /// Secret, single-use activation/reset code to deliver to the account owner.
    /// Redeem it through POST /v1/auth/reset-password; never log or persist it in plaintext.
    pub code: String,
    /// UTC deadline for redeeming the code, 30 minutes after it was issued.
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Redeem a local login activation/reset code and choose a password.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct ResetLocalPasswordRequest {
    /// Optional replacement display name. Omit or use null to keep the current
    /// name; send an empty string to clear it. The assigned username is unchanged.
    #[schema(example = "Alice")]
    pub display_name: Option<String>,
    /// Unexpired, single-use code issued during activation, reset, or operator recovery.
    pub code: String,
    /// At least 15 Unicode characters and at most 1024 UTF-8 bytes.
    #[schema(example = "a new personal passphrase")]
    pub new_password: String,
}
