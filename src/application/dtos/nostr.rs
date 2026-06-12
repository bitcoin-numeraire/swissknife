use std::collections::HashMap;

use nostr_sdk::PublicKey;
use serde::Serialize;
use utoipa::ToSchema;

pub use swissknife_types::NostrNIP05QueryParams;

#[derive(Debug, Serialize, ToSchema)]
pub struct NostrNIP05Response {
    /// Found names
    pub names: HashMap<String, String>,
}

impl NostrNIP05Response {
    pub fn new(name: String, pubkey: PublicKey) -> Self {
        let mut names = HashMap::new();
        names.insert(name, pubkey.to_string());
        Self { names }
    }
}
