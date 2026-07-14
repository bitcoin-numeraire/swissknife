use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use uuid::Uuid;

use swissknife_types::{
    Account, ApiKey, Balance, BtcAddress, CreateAccountRequest, CreateApiKeyRequest, CreateWalletRequest,
    NewBtcAddressRequest, Permission, Wallet,
};

use super::chain;
use super::client::Auth;
use super::wait::wait_until;
use super::TestApp;

static COUNTER: AtomicU64 = AtomicU64::new(0);
const MAINNET_BTC_ASSET_ID: &str = "00000000-0000-4000-8000-000000000001";
const REGTEST_BTC_ASSET_ID: &str = "00000000-0000-4000-8000-000000000004";
const SIGNET_BTC_ASSET_ID: &str = "00000000-0000-4000-8000-000000000006";

/// A process-unique, lowercase identifier with the given prefix, so tests
/// sharing one instance never collide on identity subjects, usernames, etc.
pub fn unique(prefix: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{n}", std::process::id()).to_ascii_lowercase()
}

impl TestApp {
    /// Create an administrator-managed account without a login identity.
    pub async fn create_account(&self, token: &str, display_name: &str) -> Account {
        self.create_account_with_permissions(token, display_name, vec![]).await
    }

    /// Create an administrator-managed account with the requested stored permissions.
    pub async fn create_account_with_permissions(
        &self,
        token: &str,
        display_name: &str,
        permissions: Vec<Permission>,
    ) -> Account {
        let res = self
            .api()
            .post(
                "/v1/accounts",
                Auth::Bearer(token),
                CreateAccountRequest {
                    display_name: Some(display_name.to_string()),
                    permissions,
                },
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "create account failed: {}", res.body);
        res.parse::<Account>()
    }

    /// Provision a fresh account and return its regtest BTC wallet. Behavioural
    /// tests use their own account wallet so balances stay isolated on the
    /// shared instance.
    pub async fn create_wallet(&self, token: &str, label: &str) -> Wallet {
        let account = self.create_account(token, label).await;

        let res = self
            .api()
            .post(
                "/v1/wallets",
                Auth::Bearer(token),
                CreateWalletRequest {
                    account_id: Some(account.id),
                    asset_id: regtest_btc_asset_id(),
                },
            )
            .await;
        assert_eq!(
            res.status.as_u16(),
            200,
            "create_wallet /v1/wallets failed: {}",
            res.body
        );
        res.parse::<Wallet>()
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
    /// via `/v1/me/api-keys` (which fills in the account). Returns the raw
    /// secret for use as `Auth::ApiKey` — a credential narrower than the admin
    /// JWT, for exercising permission enforcement.
    pub async fn api_key(&self, token: &str, permissions: Vec<Permission>) -> String {
        let res = self
            .api()
            .post(
                "/v1/me/api-keys",
                Auth::Bearer(token),
                CreateApiKeyRequest {
                    account_id: None,
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

    /// Mint an API key for `account_id` via the admin endpoint with
    /// `permissions`, returning its secret. The key authenticates into that
    /// account.
    pub async fn account_api_key(&self, token: &str, account_id: Uuid, permissions: Vec<Permission>) -> String {
        let res = self
            .api()
            .post(
                "/v1/api-keys",
                Auth::Bearer(token),
                CreateApiKeyRequest {
                    account_id: Some(account_id),
                    name: unique("account-key"),
                    permissions,
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "mint account key failed: {}", res.body);
        res.parse::<ApiKey>()
            .key
            .expect("a freshly created key returns its secret")
    }

    /// Create a fresh account, a full-permission API key, and its regtest BTC
    /// wallet through public HTTP endpoints for exercising `/v1/me` routes.
    pub async fn create_account_with_wallet(&self, token: &str, label: &str) -> TestAccount {
        let account = self.create_account(token, label).await;
        let key = self
            .account_api_key(token, account.id, Permission::all_permissions())
            .await;
        let res = self
            .api()
            .post(
                "/v1/me/wallets",
                Auth::ApiKey(&key),
                CreateWalletRequest {
                    account_id: None,
                    asset_id: regtest_btc_asset_id(),
                },
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "create account wallet failed: {}", res.body);
        let wallet = res.parse::<Wallet>();
        TestAccount { account, wallet, key }
    }
}

/// A distinct account with an API-key credential and regtest BTC wallet.
pub struct TestAccount {
    pub account: Account,
    pub wallet: Wallet,
    pub key: String,
}

pub fn regtest_btc_asset_id() -> Uuid {
    seeded_asset_id(REGTEST_BTC_ASSET_ID)
}

pub fn mainnet_btc_asset_id() -> Uuid {
    seeded_asset_id(MAINNET_BTC_ASSET_ID)
}

pub fn signet_btc_asset_id() -> Uuid {
    seeded_asset_id(SIGNET_BTC_ASSET_ID)
}

fn seeded_asset_id(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("seeded BTC asset ID is valid")
}
