//! `/v1/wallets` — admin wallet management. Permission-gated (`*:wallet`).

use reqwest::StatusCode;
use serde_json::json;

use swissknife_types::{RegisterWalletRequest, Wallet};

use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth};

mod register_wallet {
    use super::*;

    #[tokio::test]
    async fn succeeds_and_is_persisted() {
        let app = app().await;
        let token = app.admin_token().await;
        let user_id = unique("wallet-user");

        let res = app
            .api()
            .post(
                "/v1/wallets",
                Auth::Bearer(token),
                RegisterWalletRequest {
                    user_id: user_id.clone(),
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let wallet = res.parse::<Wallet>();
        assert_eq!(wallet.user_id, user_id);
        assert_eq!(wallet.balance.available_msat, 0);

        // Persisted: fetchable by id.
        let got = app
            .api()
            .get(&format!("/v1/wallets/{}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&got, StatusCode::OK);
        assert_eq!(got.parse::<Wallet>().id, wallet.id);
    }

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app
            .api()
            .post(
                "/v1/wallets",
                Auth::None,
                RegisterWalletRequest {
                    user_id: unique("wallet-user"),
                },
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_a_malformed_body() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app.api().post("/v1/wallets", Auth::Bearer(token), json!({})).await;
        assert!(
            matches!(res.status, StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST),
            "expected 4xx for malformed body, got {} ({})",
            res.status,
            res.body
        );
    }
}

mod get_wallet {
    use super::*;

    #[tokio::test]
    async fn unknown_id_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .get(&format!("/v1/wallets/{}", uuid::Uuid::new_v4()), Auth::Bearer(token))
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }
}
