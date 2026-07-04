//! Permission enforcement (403): every admin endpoint rejects a credential that
//! carries every scope except the one it requires. Authentication (401) is
//! covered by the `auth` suite and each domain suite's `requires_authentication`;
//! `/v1/me/*` runs no permission check (it is scoped to the caller's own wallet).

use reqwest::{Method, StatusCode};
use serde_json::Value;

use swissknife_types::{
    CreateApiKeyRequest, CreateWalletRequest, NewBtcAddressRequest, NewInvoiceRequest, Permission,
    RegisterLnAddressRequest, SendPaymentRequest, UpdateLnAddressRequest,
};

use crate::common::fixtures::unique;
use crate::common::{app, Auth};

type Case = (Method, String, Permission, Option<Value>);

fn body<T: serde::Serialize>(value: T) -> Option<Value> {
    Some(serde_json::to_value(value).expect("request body serializes"))
}

/// The standard CRUD endpoints of an admin resource: list, get-by-id, create,
/// delete-one and delete-many, each with its read/write permission.
fn crud(prefix: &str, read: Permission, write: Permission, id: uuid::Uuid, create_body: Option<Value>) -> Vec<Case> {
    vec![
        (Method::GET, prefix.to_string(), read.clone(), None),
        (Method::GET, format!("{prefix}/{id}"), read, None),
        (Method::POST, prefix.to_string(), write.clone(), create_body),
        (Method::DELETE, format!("{prefix}/{id}"), write.clone(), None),
        (Method::DELETE, prefix.to_string(), write, None),
    ]
}

#[tokio::test]
async fn every_admin_endpoint_enforces_its_permission() {
    let app = app().await;
    let token = app.admin_token().await;
    let id = uuid::Uuid::new_v4();

    // Every permission-gated endpoint. check_permission runs before the use
    // case, so the body only has to deserialize and the {id} need not exist.
    let mut cases: Vec<Case> = Vec::new();

    cases.extend(crud(
        "/v1/wallets",
        Permission::ReadWallet,
        Permission::WriteWallet,
        id,
        body(CreateWalletRequest {
            account_id: uuid::Uuid::new_v4(),
            asset_id: uuid::Uuid::new_v4(),
        }),
    ));
    cases.push((
        Method::GET,
        "/v1/wallets/overviews".to_string(),
        Permission::ReadWallet,
        None,
    ));

    cases.extend(crud(
        "/v1/invoices",
        Permission::ReadLnTransaction,
        Permission::WriteLnTransaction,
        id,
        body(NewInvoiceRequest {
            wallet_id: None,
            amount_msat: 1_000,
            description: None,
            expiry: None,
        }),
    ));

    cases.extend(crud(
        "/v1/payments",
        Permission::ReadLnTransaction,
        Permission::WriteLnTransaction,
        id,
        body(SendPaymentRequest {
            wallet_id: None,
            input: "guard".to_string(),
            amount_msat: None,
            comment: None,
        }),
    ));

    cases.extend(crud(
        "/v1/api-keys",
        Permission::ReadApiKey,
        Permission::WriteApiKey,
        id,
        body(CreateApiKeyRequest {
            account_id: Some(uuid::Uuid::new_v4()),
            name: unique("guard"),
            permissions: vec![],
            description: None,
            expiry: None,
        }),
    ));

    cases.extend(crud(
        "/v1/lightning-addresses",
        Permission::ReadLnAddress,
        Permission::WriteLnAddress,
        id,
        body(RegisterLnAddressRequest {
            account_id: None,
            username: unique("guard"),
            allows_nostr: false,
            nostr_pubkey: None,
        }),
    ));
    cases.push((
        Method::PUT,
        format!("/v1/lightning-addresses/{id}"),
        Permission::WriteLnAddress,
        body(UpdateLnAddressRequest {
            username: None,
            active: Some(false),
            allows_nostr: None,
            nostr_pubkey: None,
        }),
    ));

    cases.extend(crud(
        "/v1/bitcoin/addresses",
        Permission::ReadBtcAddress,
        Permission::WriteBtcAddress,
        id,
        body(NewBtcAddressRequest {
            wallet_id: None,
            address_type: None,
        }),
    ));

    for (method, path, required, payload) in cases {
        let scopes: Vec<Permission> = Permission::all_permissions()
            .into_iter()
            .filter(|p| p != &required)
            .collect();
        let key = app.api_key(token, scopes).await;

        let res = app
            .api()
            .request(method.clone(), &path, Auth::ApiKey(&key), payload)
            .await;

        assert_eq!(
            res.status,
            StatusCode::FORBIDDEN,
            "{method} {path} must require {required:?}; got {} ({})",
            res.status,
            res.body
        );
    }
}
