use serde::{Deserialize, Serialize};

use super::permission::Permission;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthClaims {
    pub exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: usize, // Optional. Issued at (as UTC timestamp)
    pub sub: String,
    pub permissions: Vec<Permission>,
}
