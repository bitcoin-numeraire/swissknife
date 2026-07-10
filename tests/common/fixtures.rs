use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
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
const REGTEST_BTC_ASSET_ID: &str = "00000000-0000-4000-8000-000000000004";

/// A process-unique, lowercase identifier with the given prefix, so tests
/// sharing one instance never collide on wallet user ids, usernames, etc.
pub fn unique(prefix: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{n}", std::process::id()).to_ascii_lowercase()
}

impl TestApp {
    /// Create an administrator-managed account without a login identity.
    pub async fn create_account(&self, token: &str, display_name: &str) -> Account {
        let res = self
            .api()
            .post(
                "/v1/accounts",
                Auth::Bearer(token),
                CreateAccountRequest {
                    display_name: Some(display_name.to_string()),
                    permissions: vec![],
                },
            )
            .await;
        assert_eq!(res.status.as_u16(), 200, "create account failed: {}", res.body);
        res.parse::<Account>()
    }

    /// Provision a fresh account and return its regtest BTC wallet. Behavioural
    /// tests use their own account wallet so balances stay isolated on the
    /// shared instance.
    pub async fn create_wallet(&self, token: &str, _label: &str) -> Wallet {
        let account_id = Uuid::new_v4();
        self.create_account_fixture(account_id).await;

        let res = self
            .api()
            .post(
                "/v1/wallets",
                Auth::Bearer(token),
                CreateWalletRequest {
                    account_id: Some(account_id),
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

    /// Mint an API key for `account_id` via the admin endpoint with `permissions`,
    /// returning its secret. The key authenticates into that account.
    pub async fn user_api_key(&self, token: &str, account_id: Uuid, permissions: Vec<Permission>) -> String {
        self.create_account_fixture(account_id).await;

        let res = self
            .api()
            .post(
                "/v1/api-keys",
                Auth::Bearer(token),
                CreateApiKeyRequest {
                    account_id: Some(account_id),
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

    async fn create_account_fixture(&self, account_id: Uuid) {
        let db = Database::connect(&self.database_url)
            .await
            .expect("connect to integration database");
        let backend = db.get_database_backend();
        let identity_id = uuid_literal(backend, Uuid::new_v4());
        let subject = sql_string_literal(&format!("fixture:{account_id}"));
        let account_id = uuid_literal(backend, account_id);

        for sql in [
            format!(
                "INSERT INTO account (id, permissions, created_at) \
                 VALUES ({account_id}, '[]', CURRENT_TIMESTAMP) \
                 ON CONFLICT (id) DO NOTHING"
            ),
            format!(
                "INSERT INTO auth_identity (id, account_id, provider, subject, created_at) \
                 VALUES ({identity_id}, {account_id}, 'jwt', {subject}, CURRENT_TIMESTAMP) \
                 ON CONFLICT (provider, subject) DO NOTHING"
            ),
            format!(
                "INSERT INTO account_preference (account_id, dashboard_settings, created_at) \
                 VALUES ({account_id}, '{{}}', CURRENT_TIMESTAMP) \
                 ON CONFLICT (account_id) DO NOTHING"
            ),
        ] {
            db.execute(Statement::from_string(backend, sql))
                .await
                .expect("seed integration account fixture");
        }
    }

    /// Register a fresh wallet and a full-permission API key for it: a distinct
    /// external account for exercising the `/me` endpoints.
    pub async fn create_user(&self, token: &str, _label: &str) -> TestUser {
        let account_id = Uuid::new_v4();
        let key = self
            .user_api_key(token, account_id, Permission::all_permissions())
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
        assert_eq!(
            res.status.as_u16(),
            200,
            "create_user /v1/me/wallets failed: {}",
            res.body
        );
        let wallet = res.parse::<Wallet>();
        TestUser { wallet, key }
    }
}

/// A distinct user (own wallet) with an API-key credential, for `/me` tests.
pub struct TestUser {
    pub wallet: Wallet,
    pub key: String,
}

fn uuid_literal(backend: DatabaseBackend, id: Uuid) -> String {
    match backend {
        DatabaseBackend::Sqlite => format!("X'{}'", id.as_simple()),
        DatabaseBackend::Postgres => format!("'{id}'::uuid"),
        DatabaseBackend::MySql => format!("'{id}'"),
    }
}

fn regtest_btc_asset_id() -> Uuid {
    Uuid::parse_str(REGTEST_BTC_ASSET_ID).expect("seeded regtest BTC asset ID is valid")
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
