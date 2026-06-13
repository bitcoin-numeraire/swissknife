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
        assert_error(&res, StatusCode::BAD_REQUEST);
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

mod list {
    use super::*;

    #[tokio::test]
    async fn includes_the_registered_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "wallet-list").await;

        // A large limit keeps the wallet on the page regardless of how many
        // others share the instance.
        let res = app.api().get("/v1/wallets?limit=1000", Auth::Bearer(token)).await;
        assert_status(&res, StatusCode::OK);
        assert!(
            res.parse::<Vec<Wallet>>().iter().any(|w| w.id == wallet.id),
            "the registered wallet is listed"
        );
    }
}

mod delete {
    use super::*;

    #[tokio::test]
    async fn deletes_a_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "wallet-del").await;

        let del = app
            .api()
            .delete(&format!("/v1/wallets/{}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&del, StatusCode::OK);

        let gone = app
            .api()
            .get(&format!("/v1/wallets/{}", wallet.id), Auth::Bearer(token))
            .await;
        assert_error(&gone, StatusCode::NOT_FOUND);
    }
}
