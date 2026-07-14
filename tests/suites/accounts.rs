//! `/v1/accounts` administrative account lifecycle and permission enforcement.

use reqwest::StatusCode;

use swissknife_types::{
    Account, CreateAccountRequest, CreateWalletRequest, Permission, RegisterLnAddressRequest,
    UpdateAccountPermissionsRequest, UpdateAccountRequest, Wallet,
};

use crate::common::fixtures::{regtest_btc_asset_id, unique};
use crate::common::{app, assert_error, assert_status, Auth};

mod lifecycle {
    use super::*;

    #[tokio::test]
    async fn creates_lists_updates_permissions_and_deletes_an_account_aggregate() {
        let app = app().await;
        let admin = app.admin_token().await;
        let display_name = unique("account-lifecycle");
        let request = CreateAccountRequest {
            display_name: Some(display_name.clone()),
            permissions: vec![Permission::ReadWallet],
        };

        let created = app.api().post("/v1/accounts", Auth::Bearer(admin), request).await;
        assert_status(&created, StatusCode::OK);
        let created = created.parse::<Account>();
        assert_eq!(created.display_name.as_deref(), Some(display_name.as_str()));
        assert!(created.identity.is_none());
        assert_eq!(created.permissions.as_deref(), Some(&[Permission::ReadWallet][..]));

        let listed = app.api().get("/v1/accounts?limit=1000", Auth::Bearer(admin)).await;
        assert_status(&listed, StatusCode::OK);
        assert!(listed
            .parse::<Vec<Account>>()
            .iter()
            .any(|account| account.id == created.id));

        let fetched = app
            .api()
            .get(&format!("/v1/accounts/{}", created.id), Auth::Bearer(admin))
            .await;
        assert_status(&fetched, StatusCode::OK);
        assert_eq!(fetched.parse::<Account>().id, created.id);

        let updated = app
            .api()
            .put(
                &format!("/v1/accounts/{}", created.id),
                Auth::Bearer(admin),
                UpdateAccountRequest {
                    display_name: Some("Updated account".to_string()),
                },
            )
            .await;
        assert_status(&updated, StatusCode::OK);
        assert_eq!(
            updated.parse::<Account>().display_name.as_deref(),
            Some("Updated account")
        );

        let permissions = app
            .api()
            .put(
                &format!("/v1/accounts/{}/permissions", created.id),
                Auth::Bearer(admin),
                UpdateAccountPermissionsRequest {
                    permissions: vec![Permission::ReadWallet, Permission::WriteWallet, Permission::ReadWallet],
                },
            )
            .await;
        assert_status(&permissions, StatusCode::OK);
        assert_eq!(
            permissions.parse::<Account>().permissions.as_deref(),
            Some(&[Permission::ReadWallet, Permission::WriteWallet][..])
        );

        let wallet = app
            .api()
            .post(
                "/v1/wallets",
                Auth::Bearer(admin),
                CreateWalletRequest {
                    account_id: Some(created.id),
                    asset_id: regtest_btc_asset_id(),
                },
            )
            .await;
        assert_status(&wallet, StatusCode::OK);
        let wallet = wallet.parse::<Wallet>();
        let lightning_address = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(admin),
                RegisterLnAddressRequest {
                    account_id: Some(created.id),
                    username: unique("deleted-account"),
                    allows_nostr: false,
                    nostr_pubkey: None,
                },
            )
            .await;
        assert_status(&lightning_address, StatusCode::OK);
        let lightning_address = lightning_address.parse::<swissknife_types::LnAddress>();
        let account_key = app
            .account_api_key(admin, created.id, Permission::all_permissions())
            .await;

        let deleted = app
            .api()
            .delete(&format!("/v1/accounts/{}", created.id), Auth::Bearer(admin))
            .await;
        assert_status(&deleted, StatusCode::OK);

        let missing_account = app
            .api()
            .get(&format!("/v1/accounts/{}", created.id), Auth::Bearer(admin))
            .await;
        assert_error(&missing_account, StatusCode::NOT_FOUND);
        let missing_wallet = app
            .api()
            .get(&format!("/v1/wallets/{}", wallet.id), Auth::Bearer(admin))
            .await;
        assert_error(&missing_wallet, StatusCode::NOT_FOUND);
        let missing_address = app
            .api()
            .get(
                &format!("/v1/lightning-addresses/{}", lightning_address.id),
                Auth::Bearer(admin),
            )
            .await;
        assert_error(&missing_address, StatusCode::NOT_FOUND);
        let revoked_key = app.api().get("/v1/me", Auth::ApiKey(&account_key)).await;
        assert_error(&revoked_key, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bulk_deletes_selected_accounts() {
        let app = app().await;
        let admin = app.admin_token().await;
        let first = app.create_account(admin, &unique("account")).await;
        let second = app.create_account(admin, &unique("account")).await;

        let deleted = app
            .api()
            .delete(
                &format!("/v1/accounts?ids={}&ids={}", first.id, second.id),
                Auth::Bearer(admin),
            )
            .await;

        assert_status(&deleted, StatusCode::OK);
        assert_eq!(deleted.parse::<u64>(), 2);
    }

    #[tokio::test]
    async fn cannot_delete_the_authenticated_account() {
        let app = app().await;
        let admin = app.admin_token().await;
        let account = app.api().get("/v1/me", Auth::Bearer(admin)).await.parse::<Account>();

        let response = app
            .api()
            .delete(&format!("/v1/accounts/{}", account.id), Auth::Bearer(admin))
            .await;

        assert_error(&response, StatusCode::CONFLICT);
    }
}

mod permissions {
    use super::*;

    #[tokio::test]
    async fn read_and_write_scopes_are_enforced_independently() {
        let app = app().await;
        let admin = app.admin_token().await;
        let read_key = app.api_key(admin, vec![Permission::ReadAccount]).await;
        let write_key = app.api_key(admin, vec![Permission::WriteAccount]).await;

        let list = app.api().get("/v1/accounts?limit=1", Auth::ApiKey(&read_key)).await;
        assert_status(&list, StatusCode::OK);
        let create_with_read = app
            .api()
            .post(
                "/v1/accounts",
                Auth::ApiKey(&read_key),
                CreateAccountRequest {
                    display_name: None,
                    permissions: vec![],
                },
            )
            .await;
        assert_error(&create_with_read, StatusCode::FORBIDDEN);

        let created = app
            .api()
            .post(
                "/v1/accounts",
                Auth::ApiKey(&write_key),
                CreateAccountRequest {
                    display_name: None,
                    permissions: vec![],
                },
            )
            .await;
        assert_status(&created, StatusCode::OK);
        let list_with_write = app.api().get("/v1/accounts?limit=1", Auth::ApiKey(&write_key)).await;
        assert_error(&list_with_write, StatusCode::FORBIDDEN);
    }
}
