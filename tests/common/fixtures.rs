use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use uuid::Uuid;

use swissknife_types::{
    ApiKey, Balance, BtcAddress, CreateApiKeyRequest, NewBtcAddressRequest, Permission, RegisterWalletRequest, Wallet,
};

use super::chain;
use super::client::Auth;
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
    pub async fn create_wallet(&self, token: &str, label: &str) -> Wallet {
        let res = self
            .api()
            .post(
                "/v1/wallets",
                Auth::Bearer(token),
                RegisterWalletRequest { user_id: unique(label) },
            )
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
        res.parse::<Wallet>().balance
    }

    /// Fund `wallet_id` via an on-chain deposit and wait for SwissKnife to
    /// credit it (exercises the real deposit + sync path).
    pub async fn fund_onchain(&self, token: &str, wallet_id: Uuid, sats: u64) {
        let res = self
            .api()
            .post(
                "/v1/bitcoin/addresses",
                Auth::Bearer(token),
                NewBtcAddressRequest {
                    wallet_id: Some(wallet_id),
                    address_type: None,
                },
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "new btc address failed: {}", res.body);
        let address = res.parse::<BtcAddress>().address;

        chain::send_to_address(&address, sats).await;
        chain::mine(6).await;

        let target = sats as i64 * 1000;
        wait_until(Duration::from_secs(180), "on-chain deposit credited", || async {
            self.wallet_balance(token, wallet_id).await.available_msat >= target
        })
        .await;
    }

    /// Mint an API key for the caller's own wallet with exactly `permissions`,
    /// via `/v1/me/api-keys` (which fills in the user). Returns the raw secret
    /// for use as `Auth::ApiKey` — a credential narrower than the admin JWT, for
    /// exercising permission enforcement.
    pub async fn api_key(&self, token: &str, permissions: Vec<Permission>) -> String {
        let res = self
            .api()
            .post(
                "/v1/me/api-keys",
                Auth::Bearer(token),
                CreateApiKeyRequest {
                    user_id: None,
                    name: unique("key"),
                    permissions,
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "mint api key failed: {}", res.body);
        res.parse::<ApiKey>()
            .key
            .expect("a freshly created key returns its secret")
    }

    /// Mint an API key for `user_id` via the admin endpoint with `permissions`,
    /// returning its secret. The key authenticates as that user.
    pub async fn user_api_key(&self, token: &str, user_id: &str, permissions: Vec<Permission>) -> String {
        let res = self
            .api()
            .post(
                "/v1/api-keys",
                Auth::Bearer(token),
                CreateApiKeyRequest {
                    user_id: Some(user_id.to_string()),
                    name: unique("user-key"),
                    permissions,
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "mint user key failed: {}", res.body);
        res.parse::<ApiKey>()
            .key
            .expect("a freshly created key returns its secret")
    }

    /// Register a fresh wallet and a full-permission API key for it: a distinct
    /// external user for exercising the `/me` endpoints.
    pub async fn create_user(&self, token: &str, label: &str) -> TestUser {
        let wallet = self.create_wallet(token, label).await;
        let key = self
            .user_api_key(token, &wallet.user_id, Permission::all_permissions())
            .await;
        TestUser { wallet, key }
    }
}

/// A distinct user (own wallet) with an API-key credential, for `/me` tests.
pub struct TestUser {
    pub wallet: Wallet,
    pub key: String,
}
