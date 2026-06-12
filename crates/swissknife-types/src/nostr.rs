use std::collections::HashMap;

use nostr::PublicKey;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

/// Nostr NIP-05 query parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct NostrNIP05QueryParams {
    /// Username to query
    #[serde(default)]
    pub name: String,
}

/// Nostr NIP-05 response. Maps each queried name to its hex-encoded public key.
#[derive(Debug, serde::Serialize, ToSchema)]
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
