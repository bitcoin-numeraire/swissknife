use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::json;
use uuid::Uuid;

use super::chain;
use super::client::Auth;
use super::models::{Balance, BtcAddress, WalletResponse};
use super::wait::wait_until;
use super::TestApp;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A process-unique, lowercase identifier with the given prefix, so tests
/// sharing one instance never collide on wallet user ids, usernames, etc.
pub fn unique(prefix: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{n}", std::process::id()).to_ascii_lowercase()
}

impl TestApp {
    /// Register a fresh wallet (its own user id) and return it. Behavioural
    /// tests use their own wallet so balances stay isolated on the shared
    /// instance.
    pub async fn create_wallet(&self, token: &str, label: &str) -> WalletResponse {
        let res = self
            .api()
            .post("/v1/wallets", Auth::Bearer(token), json!({ "user_id": unique(label) }))
            .await;
        assert_eq!(res.status.as_u16(), 200, "create_wallet failed: {}", res.body);
        res.parse()
    }

    /// Current balance of `wallet_id`.
    pub async fn wallet_balance(&self, token: &str, wallet_id: Uuid) -> Balance {
        let res = self
            .api()
            .get(&format!("/v1/wallets/{wallet_id}"), Auth::Bearer(token))
            .await;
        assert_eq!(res.status.as_u16(), 200, "get wallet {wallet_id} failed: {}", res.body);
        res.parse::<WalletResponse>().balance
    }

    /// Fund `wallet_id` via an on-chain deposit and wait for SwissKnife to
    /// credit it (exercises the real deposit + sync path).
    pub async fn fund_onchain(&self, token: &str, wallet_id: Uuid, sats: u64) {
        let res = self
            .api()
            .post(
                "/v1/bitcoin/addresses",
                Auth::Bearer(token),
                json!({ "wallet_id": wallet_id, "address_type": "p2wpkh" }),
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "new btc address failed: {}", res.body);
        let address = res.parse::<BtcAddress>().address;

        chain::send_to_address(&address, sats).await;
        chain::mine(6).await;

        let target = sats as i64 * 1000;
        wait_until(Duration::from_secs(90), "on-chain deposit credited", || async {
            self.wallet_balance(token, wallet_id).await.available_msat >= target
        })
        .await;
    }
}
