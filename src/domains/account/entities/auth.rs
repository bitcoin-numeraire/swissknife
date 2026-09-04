use serde::{Deserialize, Serialize};

use super::Permission;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthClaims {
    pub exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: usize, // Optional. Issued at (as UTC timestamp)
    pub sub: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_identity_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_revision: Option<uuid::Uuid>,
    pub permissions: Vec<Permission>,
}
