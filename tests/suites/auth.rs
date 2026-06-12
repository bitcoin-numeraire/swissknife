//! `/v1/auth/*` (local JWT) plus auth-middleware enforcement on protected routes.

use reqwest::StatusCode;
use serde_json::json;

use crate::common::harness::ADMIN_PASSWORD;
use crate::common::{app, assert_error, assert_status, Auth};

mod sign_in {
    use super::*;

    #[tokio::test]
    async fn with_the_correct_password_returns_a_token() {
        let app = app().await;
        app.admin_token().await; // ensure the admin exists
        let res = app
            .api()
            .post("/v1/auth/sign-in", Auth::None, json!({ "password": ADMIN_PASSWORD }))
            .await;
        assert_status(&res, StatusCode::OK);
        assert!(res.body["token"].as_str().is_some(), "{}", res.body);
    }

    #[tokio::test]
    async fn with_a_wrong_password_is_unauthorized() {
        let app = app().await;
        app.admin_token().await;
        let res = app
            .api()
            .post("/v1/auth/sign-in", Auth::None, json!({ "password": "wrong-password" }))
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
                json!({ "password": "another-password" }),
            )
            .await;
        assert_error(&res, StatusCode::CONFLICT);
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
