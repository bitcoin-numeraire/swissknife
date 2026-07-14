//! OAuth2/OIDC test harness: a SwissKnife instance configured with
//! `auth_provider = oauth2`, pointed at the dockerized mock OIDC provider
//! ([navikt/mock-oauth2-server]). It is shared across the oauth2 suite via a
//! `OnceCell`, mirroring the JWT [`app()`](super::harness::app).
//!
//! The binary and the tests both reach the IdP at the same `127.0.0.1:<port>`,
//! so the issuer SwissKnife discovers (and validates `iss` against) matches the
//! `iss` the IdP stamps into the tokens the tests fetch. Token claim sets are
//! selected per request `client_id`, shaped by
//! `tests/itest/config/mock-oauth2/config.json`.

use reqwest::Client;
use serde_json::Value;
use tokio::sync::OnceCell;

use super::client::ApiClient;
use super::harness::{matrix_cell, spawn_instance};

/// Audience the IdP mints into tokens and the instance is configured to require.
/// Kept in sync with `tests/itest/config/mock-oauth2/config.json`.
pub const AUDIENCE: &str = "https://swissknife.itest/api";

/// Default OIDC issuer id (path segment) served by the mock IdP.
pub const ISSUER_ID: &str = "default";

/// `client_id`s the IdP config maps to specific claim sets.
pub const CLIENT_FULL: &str = "itest-full"; // all permissions
pub const CLIENT_READONLY: &str = "itest-readonly"; // read:wallet only
pub const CLIENT_CONCURRENT: &str = "itest-concurrent"; // fresh provisioning subject
pub const CLIENT_WRONG_AUD: &str = "itest-wrong-aud"; // mismatched audience

/// Host:port at which both the binary and the tests reach the IdP, so the
/// discovered issuer and the token `iss` agree. Maps to the compose service's
/// published port (see `docker-compose.yml`); overridable for non-default stacks.
fn idp_base() -> String {
    let port = std::env::var("SWISSKNIFE_ITEST_OAUTH2_PORT").unwrap_or_else(|_| "8090".to_string());
    format!("http://127.0.0.1:{port}")
}

static OAUTH2_APP: OnceCell<OAuth2App> = OnceCell::const_new();

/// The process-wide shared OAuth2-configured instance, spawned on first use.
pub async fn oauth2_app() -> &'static OAuth2App {
    OAUTH2_APP.get_or_init(OAuth2App::start).await
}

pub struct OAuth2App {
    pub base_url: String,
    idp_base: String,
}

impl OAuth2App {
    async fn start() -> OAuth2App {
        let (database, provider) = matrix_cell();
        let idp_base = idp_base();
        // The instance runs OIDC discovery at startup against this issuer, so the
        // mock IdP must already be serving (brought up by `make itest-up`); if it
        // is not, startup fails and the readiness wait surfaces the log tail.
        let issuer = format!("{idp_base}/{ISSUER_ID}");

        let spawned = spawn_instance(
            &database,
            &provider,
            &format!("{database}-{provider}-oauth2"),
            &[
                ("SWISSKNIFE_AUTH_PROVIDER", "oauth2".to_string()),
                // `domain` is the issuer base URL; an explicit scheme keeps the
                // binary talking http to the local IdP (see OAuth2Config).
                ("SWISSKNIFE_OAUTH2__DOMAIN", issuer),
                ("SWISSKNIFE_OAUTH2__AUDIENCE", AUDIENCE.to_string()),
            ],
        )
        .await;

        OAuth2App {
            base_url: spawned.base_url,
            idp_base,
        }
    }

    /// A fresh HTTP client bound to this instance.
    pub fn api(&self) -> ApiClient {
        ApiClient::new(self.base_url.clone())
    }

    /// Fetch a signed access token from the default issuer for `client_id`
    /// (which selects the claim set via the IdP config).
    pub async fn token(&self, client_id: &str) -> String {
        self.token_from(ISSUER_ID, client_id).await
    }

    /// Fetch a signed access token from an arbitrary `issuer_id` — used to mint
    /// a token whose `iss` does not match the one the instance trusts.
    pub async fn token_from(&self, issuer_id: &str, client_id: &str) -> String {
        let url = format!("{}/{issuer_id}/token", self.idp_base);
        // A fresh client per call: the shared instance is reused across tests,
        // each on its own Tokio runtime, and a `reqwest::Client` is bound to the
        // runtime that created it (mirrors `ApiClient`).
        let res = Client::new()
            .post(&url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", client_id),
                ("client_secret", "itest-secret"),
                ("scope", "openid"),
            ])
            .send()
            .await
            .expect("reach the mock IdP token endpoint");
        let status = res.status();
        let body: Value = res.json().await.expect("token response is JSON");
        body["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("token response has no access_token (status {status}): {body}"))
            .to_string()
    }
}
