//! `/v1/accounts` administrator-managed account CRUD.

use reqwest::StatusCode;

use swissknife_types::{
    Account, CreateAccountRequest, Permission, UpdateAccountPermissionsRequest, UpdateAccountRequest,
};

use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth};

#[tokio::test]
async fn manages_an_account_without_a_login_identity() {
    let app = app().await;
    let token = app.admin_token().await;
    let display_name = unique("account");

    let created = app
        .api()
        .post(
            "/v1/accounts",
            Auth::Bearer(token),
            CreateAccountRequest {
                display_name: Some(display_name.clone()),
                permissions: vec![Permission::ReadWallet],
            },
        )
        .await;
    assert_status(&created, StatusCode::OK);
    let created = created.parse::<Account>();
    assert_eq!(created.display_name.as_deref(), Some(display_name.as_str()));
    assert_eq!(created.permissions, Some(vec![Permission::ReadWallet]));
    assert!(created.identity.is_none());

    let updated = app
        .api()
        .put(
            &format!("/v1/accounts/{}", created.id),
            Auth::Bearer(token),
            UpdateAccountRequest {
                display_name: Some("Treasury".to_string()),
            },
        )
        .await;
    assert_status(&updated, StatusCode::OK);
    assert_eq!(updated.parse::<Account>().display_name.as_deref(), Some("Treasury"));

    let permissions = app
        .api()
        .put(
            &format!("/v1/accounts/{}/permissions", created.id),
            Auth::Bearer(token),
            UpdateAccountPermissionsRequest {
                permissions: vec![Permission::ReadAccount, Permission::WriteWallet],
            },
        )
        .await;
    assert_status(&permissions, StatusCode::OK);
    assert_eq!(
        permissions.parse::<Account>().permissions,
        Some(vec![Permission::ReadAccount, Permission::WriteWallet])
    );

    let listed = app.api().get("/v1/accounts", Auth::Bearer(token)).await;
    assert_status(&listed, StatusCode::OK);
    assert!(listed
        .parse::<Vec<Account>>()
        .iter()
        .any(|account| account.id == created.id));

    let deleted = app
        .api()
        .delete(&format!("/v1/accounts/{}", created.id), Auth::Bearer(token))
        .await;
    assert_status(&deleted, StatusCode::OK);

    let missing = app
        .api()
        .get(&format!("/v1/accounts/{}", created.id), Auth::Bearer(token))
        .await;
    assert_error(&missing, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn bulk_deletes_selected_accounts() {
    let app = app().await;
    let token = app.admin_token().await;
    let first = app.create_account(token, &unique("account")).await;
    let second = app.create_account(token, &unique("account")).await;

    let deleted = app
        .api()
        .delete(
            &format!("/v1/accounts?ids={}&ids={}", first.id, second.id),
            Auth::Bearer(token),
        )
        .await;

    assert_status(&deleted, StatusCode::OK);
    assert_eq!(deleted.parse::<u64>(), 2);
}

#[tokio::test]
async fn rejects_deleting_the_authenticated_account() {
    let app = app().await;
    let token = app.admin_token().await;
    let account = app.api().get("/v1/me", Auth::Bearer(token)).await.parse::<Account>();

    let deleted = app
        .api()
        .delete(&format!("/v1/accounts/{}", account.id), Auth::Bearer(token))
        .await;

    assert_error(&deleted, StatusCode::CONFLICT);
}
