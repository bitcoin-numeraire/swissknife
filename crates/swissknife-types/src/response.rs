//! Response types that have no entity equivalent — protocol or edge shapes
//! assembled by handlers rather than returned by a use case.

use std::collections::HashMap;

use nostr::PublicKey;
use serde::Serialize;
use utoipa::{ToResponse, ToSchema};

/// Sign In Response
#[derive(Debug, Serialize, ToSchema)]
pub struct SignInResponse {
    /// JWT token
    #[schema(example = "eyJ0eXAiOiJKV1QiLCJhbGciOiJ...")]
    pub token: String,
}

/// Nostr NIP-05 response. Maps each queried name to its hex-encoded public key.
#[derive(Debug, Serialize, ToSchema)]
pub struct NostrNIP05Response {
    /// Found names, keyed by username and valued by hex-encoded public key
    pub names: HashMap<String, String>,
}

impl NostrNIP05Response {
    pub fn new(name: String, pubkey: PublicKey) -> Self {
        let mut names = HashMap::new();
        names.insert(name, pubkey.to_string());
        Self { names }
    }
}

/// Application error response
#[derive(Debug, Serialize, ToResponse, ToSchema)]
pub struct ErrorResponse {
    /// Error status
    #[schema(example = "401 Unauthorized")]
    pub status: String,

    /// Error reason
    #[schema(example = "error message")]
    pub reason: String,
}
