//! OAuth2/OIDC authentication, end-to-end against a real mock OpenID provider
//! (navikt/mock-oauth2-server). These run against a *separate* SwissKnife
//! instance configured with `auth_provider = oauth2` (see `common::oauth2`),
//! which performs OpenID discovery and JWKS-based RS256 validation at startup.
//!
//! Together they exercise the whole OAuth2 path through real HTTP: discovery,
//! signature validation against the fetched JWKS, audience/issuer/expiry checks,
//! the `sub` -> account/wallet provisioning, and JWT-scope -> permission enforcement.

use futures_util::future::join_all;
use reqwest::StatusCode;
use serde_json::json;

use swissknife_types::{Account, RegisterLnAddressRequest, Wallet};

use crate::common::fixtures::unique;
use crate::common::oauth2::{oauth2_app, CLIENT_CONCURRENT, CLIENT_FULL, CLIENT_READONLY, CLIENT_WRONG_AUD};
use crate::common::{assert_error, assert_status, Auth};

mod accepts {
    use super::*;

    /// A token the provider signs — with the configured audience and issuer —
    /// is accepted, and its `sub` is provisioned as (and stably mapped to) an
    /// account and active-network wallet.
    #[tokio::test]
    async fn a_valid_token_and_maps_the_subject_to_a_wallet() {
        let app = oauth2_app().await;
        let token = app.token(CLIENT_FULL).await;

        let res = app.api().get("/v1/me", Auth::Bearer(&token)).await;
        assert_status(&res, StatusCode::OK);
        let account = res.parse::<Account>();
        assert_ne!(account.id, uuid::Uuid::nil(), "the token subject provisions an account");
        let wallets = app.api().get("/v1/me/wallets", Auth::Bearer(&token)).await;
        assert_status(&wallets, StatusCode::OK);
        let wallet = wallets
            .parse::<Vec<Wallet>>()
            .into_iter()
            .next()
            .expect("authentication provisions an active wallet");
        assert_eq!(wallet.account_id, account.id);

        // A second call with a fresh token for the same subject resolves to the
        // same account and wallet (provisioned once, then looked up).
        let again = app.token(CLIENT_FULL).await;
        let res = app.api().get("/v1/me", Auth::Bearer(&again)).await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(
            res.parse::<Account>().id,
            account.id,
            "the subject maps to a stable account"
        );
        let wallets = app.api().get("/v1/me/wallets", Auth::Bearer(&again)).await;
        assert_status(&wallets, StatusCode::OK);
        assert_eq!(
            wallets.parse::<Vec<Wallet>>()[0].id,
            wallet.id,
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

    #[tokio::test]
    async fn concurrent_first_requests_provision_one_account_and_wallet_without_server_errors() {
        let app = oauth2_app().await;
        let token = app.token(CLIENT_CONCURRENT).await;
        let responses = join_all((0..8).map(|_| {
            let api = app.api();
            let token = token.clone();
            async move { api.get("/v1/me", Auth::Bearer(&token)).await }
        }))
        .await;

        let mut account_id = None;
        for response in responses {
            assert_status(&response, StatusCode::OK);
            let current = response.parse::<Account>().id;
            assert_eq!(*account_id.get_or_insert(current), current);
        }

        let wallets = app.api().get("/v1/me/wallets", Auth::Bearer(&token)).await;
        assert_status(&wallets, StatusCode::OK);
        let wallets = wallets.parse::<Vec<Wallet>>();
        assert_eq!(wallets.len(), 1, "concurrent authentication provisions one wallet");
        assert_eq!(wallets[0].account_id, account_id.expect("provisioned account ID"));
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
                    account_id: None,
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
