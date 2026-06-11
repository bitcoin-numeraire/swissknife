use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use super::client::Auth;
use super::TestApp;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A process-unique, lowercase identifier with the given prefix. Used for
/// wallet user ids, lightning-address usernames, etc. so tests sharing one
/// instance never collide.
pub fn unique(prefix: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{n}", std::process::id()).to_ascii_lowercase()
}

impl TestApp {
    /// Register a wallet for a fresh unique user; returns the wallet JSON.
    pub async fn create_wallet(&self, token: &str, label: &str) -> Value {
        let res = self
            .api()
            .post("/v1/wallets", Auth::Bearer(token), json!({ "user_id": unique(label) }))
            .await;
        assert_eq!(res.status.as_u16(), 200, "create_wallet failed: {}", res.body);
        res.body
    }

    /// Register a lightning address for `wallet_id`; returns the address JSON.
    pub async fn register_ln_address(&self, token: &str, wallet_id: &str, username: &str) -> Value {
        let res = self
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                json!({ "wallet_id": wallet_id, "username": username, "allows_nostr": false }),
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "register_ln_address failed: {}", res.body);
        res.body
    }
}
