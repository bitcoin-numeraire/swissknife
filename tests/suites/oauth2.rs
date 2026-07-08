//! OAuth2/OIDC authentication, end-to-end against a real mock OpenID provider
//! (navikt/mock-oauth2-server). These run against a *separate* SwissKnife
//! instance configured with `auth_provider = oauth2` (see `common::oauth2`),
//! which performs OpenID discovery and JWKS-based RS256 validation at startup.
//!
//! Together they exercise the whole OAuth2 path through real HTTP: discovery,
//! signature validation against the fetched JWKS, audience/issuer/expiry checks,
//! the `sub` -> wallet provisioning, and JWT-scope -> permission enforcement.

use reqwest::StatusCode;
use serde_json::json;

use swissknife_types::{RegisterLnAddressRequest, Wallet};

use crate::common::fixtures::unique;
use crate::common::oauth2::{oauth2_app, CLIENT_FULL, CLIENT_READONLY, CLIENT_WRONG_AUD};
use crate::common::{assert_error, assert_status, Auth};

mod accepts {
    use super::*;

    /// A token the provider signs — with the configured audience and issuer —
    /// is accepted, and its `sub` is provisioned as (and stably mapped to) a
    /// wallet.
    #[tokio::test]
    async fn a_valid_token_and_maps_the_subject_to_a_wallet() {
        let app = oauth2_app().await;
        let token = app.token(CLIENT_FULL).await;

        let res = app.api().get("/v1/me", Auth::Bearer(&token)).await;
        assert_status(&res, StatusCode::OK);
        let wallet = res.parse::<Wallet>();
        assert_ne!(
            wallet.account_id,
            uuid::Uuid::nil(),
            "the token subject provisions an account"
        );

        // A second call with a fresh token for the same subject resolves to the
        // same wallet (provisioned once, then looked up).
        let again = app.token(CLIENT_FULL).await;
        let res = app.api().get("/v1/me", Auth::Bearer(&again)).await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(
            res.parse::<Wallet>().account_id,
            wallet.account_id,
            "the subject maps to a stable wallet"
        );
    }

    /// A scope carried in the JWT grants access to a matching read endpoint.
    #[tokio::test]
    async fn a_scoped_token_can_read() {
        let app = oauth2_app().await;
        let token = app.token(CLIENT_READONLY).await; // read:wallet

        let res = app.api().get("/v1/wallets", Auth::Bearer(&token)).await;
        assert_status(&res, StatusCode::OK);
    }
}

mod rejects {
    use super::*;

    /// A scope the JWT lacks is forbidden by the permission middleware (403),
    /// proving JWT scopes flow into authorization.
    #[tokio::test]
    async fn insufficient_permissions_with_forbidden() {
        let app = oauth2_app().await;
        let token = app.token(CLIENT_READONLY).await; // lacks write:ln_address

        let res = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(&token),
                RegisterLnAddressRequest {
                    wallet_id: None,
                    username: unique("oauth2-guard"),
                    allows_nostr: false,
                    nostr_pubkey: None,
                },
            )
            .await;
        assert_eq!(
            res.status,
            StatusCode::FORBIDDEN,
            "a missing scope must be forbidden (body: {})",
            res.body
        );
    }

    /// A validly-signed token whose audience differs from the configured one is
    /// rejected (401).
    #[tokio::test]
    async fn a_wrong_audience_token() {
        let app = oauth2_app().await;
        let token = app.token(CLIENT_WRONG_AUD).await;

        let res = app.api().get("/v1/me", Auth::Bearer(&token)).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    /// A token minted by a different issuer (not the one SwissKnife discovered)
    /// is rejected (401).
    #[tokio::test]
    async fn a_token_from_an_untrusted_issuer() {
        let app = oauth2_app().await;
        let token = app.token_from("other", CLIENT_FULL).await;

        let res = app.api().get("/v1/me", Auth::Bearer(&token)).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn missing_credentials() {
        let app = oauth2_app().await;
        let res = app.api().get("/v1/me", Auth::None).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_malformed_bearer_token() {
        let app = oauth2_app().await;
        let res = app.api().get("/v1/me", Auth::Bearer("not-a-jwt")).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }
}

/// Local password auth is unavailable under an external IdP: the `/v1/auth`
/// endpoints reject sign-up/sign-in regardless of payload.
mod local_auth_is_disabled {
    use super::*;

    #[tokio::test]
    async fn sign_up_is_unsupported() {
        let app = oauth2_app().await;
        let res = app
            .api()
            .post("/v1/auth/sign-up", Auth::None, json!({ "password": "irrelevant" }))
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn sign_in_is_unsupported() {
        let app = oauth2_app().await;
        let res = app
            .api()
            .post("/v1/auth/sign-in", Auth::None, json!({ "password": "irrelevant" }))
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn change_password_is_unsupported() {
        let app = oauth2_app().await;
        let token = app.token(CLIENT_FULL).await;
        let res = app
            .api()
            .post(
                "/v1/auth/change-password",
                Auth::Bearer(&token),
                json!({
                    "current_password": "irrelevant",
                    "new_password": "still-irrelevant"
                }),
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }
}
