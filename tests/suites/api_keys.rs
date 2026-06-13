//! `/v1/api-keys` — admin API-key management, permission-gated (`*:api_key`).
//! The admin endpoint mints a key for an explicit `user_id`; minting a key for
//! oneself is the `/v1/me/api-keys` path (the `api_key` fixture). Also exercises
//! that a minted key authenticates and carries exactly its scopes.

use reqwest::StatusCode;

use swissknife_types::{ApiKey, CreateApiKeyRequest, Permission, RegisterWalletRequest};

use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth};

fn new_key(user_id: &str, permissions: Vec<Permission>) -> CreateApiKeyRequest {
    CreateApiKeyRequest {
        user_id: Some(user_id.to_string()),
        name: unique("key"),
        permissions,
        description: None,
        expiry: None,
    }
}

mod create {
    use super::*;

    #[tokio::test]
    async fn creates_a_key_for_the_named_user_that_authenticates() {
        let app = app().await;
        let token = app.admin_token().await;
        // The admin endpoint names an explicit user, who needs a wallet for the
        // key to resolve to when it is later used.
        let wallet = app.create_wallet(token, "key-owner").await;

        let res = app
            .api()
            .post(
                "/v1/api-keys",
                Auth::Bearer(token),
                new_key(&wallet.user_id, vec![Permission::ReadWallet]),
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let created = res.parse::<ApiKey>();
        assert_eq!(created.user_id, wallet.user_id, "key belongs to the named user");
        assert_eq!(
            created.permissions,
            vec![Permission::ReadWallet],
            "key carries exactly its scopes"
        );
        let key = created.key.expect("the secret is returned once on creation");

        // The key authenticates and is allowed within its scope (read:wallet).
        let listed = app.api().get("/v1/wallets", Auth::ApiKey(&key)).await;
        assert_status(&listed, StatusCode::OK);
    }

    #[tokio::test]
    async fn requires_write_api_key_permission() {
        let app = app().await;
        let token = app.admin_token().await;
        let key = app.api_key(token, vec![Permission::ReadApiKey]).await; // read but not write

        let res = app
            .api()
            .post("/v1/api-keys", Auth::ApiKey(&key), new_key("anyone", vec![]))
            .await;
        assert_error(&res, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app
            .api()
            .post("/v1/api-keys", Auth::None, new_key("anyone", vec![]))
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }
}

mod scope {
    use super::*;

    #[tokio::test]
    async fn a_minted_key_is_forbidden_beyond_its_scope() {
        let app = app().await;
        let token = app.admin_token().await;
        let key = app.api_key(token, vec![Permission::ReadWallet]).await; // no write:wallet

        let res = app
            .api()
            .post(
                "/v1/wallets",
                Auth::ApiKey(&key),
                RegisterWalletRequest {
                    user_id: unique("blocked"),
                },
            )
            .await;
        assert_error(&res, StatusCode::FORBIDDEN);
    }
}

mod manage {
    use super::*;

    #[tokio::test]
    async fn get_list_then_revoke() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "key-managed").await;

        let created = app
            .api()
            .post(
                "/v1/api-keys",
                Auth::Bearer(token),
                new_key(&wallet.user_id, vec![Permission::ReadApiKey]),
            )
            .await
            .parse::<ApiKey>();

        let got = app
            .api()
            .get(&format!("/v1/api-keys/{}", created.id), Auth::Bearer(token))
            .await;
        assert_status(&got, StatusCode::OK);
        assert_eq!(got.parse::<ApiKey>().id, created.id);

        let list = app.api().get("/v1/api-keys", Auth::Bearer(token)).await;
        assert_status(&list, StatusCode::OK);
        assert!(
            list.parse::<Vec<ApiKey>>().iter().any(|k| k.id == created.id),
            "created key is listed"
        );

        let del = app
            .api()
            .delete(&format!("/v1/api-keys/{}", created.id), Auth::Bearer(token))
            .await;
        assert_status(&del, StatusCode::OK);

        let gone = app
            .api()
            .get(&format!("/v1/api-keys/{}", created.id), Auth::Bearer(token))
            .await;
        assert_error(&gone, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_id_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .get(&format!("/v1/api-keys/{}", uuid::Uuid::new_v4()), Auth::Bearer(token))
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }
}
