//! `/v1/auth/*` (local JWT) plus auth-middleware enforcement on protected routes.

use futures_util::future::join_all;
use reqwest::StatusCode;

use swissknife_types::{ChangePasswordRequest, SignInRequest, SignInResponse, SignUpRequest};

use crate::common::client::ApiClient;
use crate::common::harness::{matrix_cell, spawn_instance, ADMIN_PASSWORD};
use crate::common::{app, assert_error, assert_status, Auth};

mod sign_in {
    use super::*;

    #[tokio::test]
    async fn with_the_correct_password_returns_a_token() {
        let app = app().await;
        app.admin_token().await; // ensure the admin exists
        let res = app
            .api()
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                SignInRequest {
                    password: ADMIN_PASSWORD.to_string(),
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        assert!(!res.parse::<SignInResponse>().token.is_empty(), "{}", res.body);
    }

    #[tokio::test]
    async fn with_a_wrong_password_is_unauthorized() {
        let app = app().await;
        app.admin_token().await;
        let res = app
            .api()
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                SignInRequest {
                    password: "wrong-password".to_string(),
                },
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }
}

mod sign_up {
    use super::*;

    #[tokio::test]
    async fn a_second_sign_up_conflicts() {
        let app = app().await;
        app.admin_token().await; // first admin already created
        let res = app
            .api()
            .post(
                "/v1/auth/sign-up",
                Auth::None,
                SignUpRequest {
                    password: "another-password".to_string(),
                },
            )
            .await;
        assert_error(&res, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn concurrent_sign_ups_have_one_winner_and_no_server_errors() {
        let (database, provider) = matrix_cell();
        let label = format!("{database}-{provider}-auth-concurrent-sign-up");
        let spawned = spawn_instance(&database, &provider, &label, &[]).await;
        let api = ApiClient::new(spawned.base_url);

        let responses = join_all((0..8).map(|_| {
            let api = api.clone();
            async move {
                api.post(
                    "/v1/auth/sign-up",
                    Auth::None,
                    SignUpRequest {
                        password: ADMIN_PASSWORD.to_string(),
                    },
                )
                .await
            }
        }))
        .await;

        let mut winner = None;
        let mut conflicts = 0;
        for response in responses {
            match response.status {
                StatusCode::OK => winner = Some(response.parse::<SignInResponse>().token),
                StatusCode::CONFLICT => conflicts += 1,
                status => panic!("concurrent sign-up returned {status}: {}", response.body),
            }
        }
        assert!(winner.is_some(), "one sign-up must create the admin account");
        assert_eq!(conflicts, 7, "all losing sign-ups must report conflict");

        let token = winner.expect("sign-up winner token");
        let profile = api.get("/v1/me", Auth::Bearer(&token)).await;
        assert_status(&profile, StatusCode::OK);
        let wallets = api.get("/v1/me/wallets", Auth::Bearer(&token)).await;
        assert_status(&wallets, StatusCode::OK);
        assert_eq!(wallets.parse::<Vec<swissknife_types::Wallet>>().len(), 1);
    }
}

mod change_password {
    use super::*;

    #[tokio::test]
    async fn rejects_missing_credentials() {
        let app = app().await;
        let res = app
            .api()
            .post(
                "/v1/auth/change-password",
                Auth::None,
                ChangePasswordRequest {
                    current_password: ADMIN_PASSWORD.to_string(),
                    new_password: "new-integration-admin-password".to_string(),
                },
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn with_a_wrong_current_password_is_unprocessable() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .post(
                "/v1/auth/change-password",
                Auth::Bearer(token),
                ChangePasswordRequest {
                    current_password: "wrong-password".to_string(),
                    new_password: "new-integration-admin-password".to_string(),
                },
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn with_the_correct_current_password_updates_future_sign_ins() {
        let api = isolated_api("auth-change-password").await;
        let admin_token = bootstrap_admin(&api, ADMIN_PASSWORD).await;
        let new_password = "new-integration-admin-password";

        let res = api
            .post(
                "/v1/auth/change-password",
                Auth::Bearer(&admin_token),
                ChangePasswordRequest {
                    current_password: ADMIN_PASSWORD.to_string(),
                    new_password: new_password.to_string(),
                },
            )
            .await;
        assert_status(&res, StatusCode::NO_CONTENT);

        let old_password = api
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                SignInRequest {
                    password: ADMIN_PASSWORD.to_string(),
                },
            )
            .await;
        assert_error(&old_password, StatusCode::UNAUTHORIZED);

        let new_password = api
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                SignInRequest {
                    password: new_password.to_string(),
                },
            )
            .await;
        assert_status(&new_password, StatusCode::OK);
        assert!(
            !new_password.parse::<SignInResponse>().token.is_empty(),
            "{}",
            new_password.body
        );
    }

    async fn isolated_api(label: &str) -> ApiClient {
        let (database, provider) = matrix_cell();
        let label = format!("{database}-{provider}-{label}");
        let spawned = spawn_instance(&database, &provider, &label, &[]).await;
        ApiClient::new(spawned.base_url)
    }

    async fn bootstrap_admin(api: &ApiClient, password: &str) -> String {
        let res = api
            .post(
                "/v1/auth/sign-up",
                Auth::None,
                SignUpRequest {
                    password: password.to_string(),
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        res.parse::<SignInResponse>().token
    }
}

mod protected_routes {
    use super::*;

    #[tokio::test]
    async fn reject_missing_credentials() {
        let app = app().await;
        let res = app.api().get("/v1/me", Auth::None).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn reject_a_malformed_bearer_token() {
        let app = app().await;
        let res = app.api().get("/v1/me", Auth::Bearer("not-a-jwt")).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn accept_a_valid_token() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app.api().get("/v1/me", Auth::Bearer(token)).await;
        assert_status(&res, StatusCode::OK);
    }
}
