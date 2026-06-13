//! Permission enforcement (`403`): every admin endpoint rejects a credential
//! that carries every scope *except* the one it requires. Authentication (`401`)
//! is covered by the `auth` suite's middleware tests and each domain suite's
//! `requires_authentication`; `/v1/me/*` runs no permission check (it is scoped
//! to the caller's own wallet), so it is not represented here.

use reqwest::{Method, StatusCode};
use serde_json::Value;

use swissknife_types::{
    CreateApiKeyRequest, NewBtcAddressRequest, NewInvoiceRequest, Permission, RegisterLnAddressRequest,
    RegisterWalletRequest, SendPaymentRequest,
};

use crate::common::fixtures::unique;
use crate::common::{app, Auth};

fn body<T: serde::Serialize>(value: T) -> Option<Value> {
    Some(serde_json::to_value(value).expect("request body serializes"))
}

#[tokio::test]
async fn each_admin_endpoint_requires_its_permission() {
    let app = app().await;
    let token = app.admin_token().await;

    // (method, path, required permission, body for POST). The body only has to
    // deserialize: `check_permission` runs before the use case, so a key holding
    // every scope *except* `required` is forbidden (403) regardless of content.
    let cases: Vec<(Method, &str, Permission, Option<Value>)> = vec![
        (Method::GET, "/v1/wallets", Permission::ReadWallet, None),
        (
            Method::POST,
            "/v1/wallets",
            Permission::WriteWallet,
            body(RegisterWalletRequest {
                user_id: unique("guard"),
            }),
        ),
        (Method::GET, "/v1/invoices", Permission::ReadLnTransaction, None),
        (
            Method::POST,
            "/v1/invoices",
            Permission::WriteLnTransaction,
            body(NewInvoiceRequest {
                wallet_id: None,
                amount_msat: 1_000,
                description: None,
                expiry: None,
            }),
        ),
        (Method::GET, "/v1/payments", Permission::ReadLnTransaction, None),
        (
            Method::POST,
            "/v1/payments",
            Permission::WriteLnTransaction,
            body(SendPaymentRequest {
                wallet_id: None,
                input: "guard".to_string(),
                amount_msat: None,
                comment: None,
            }),
        ),
        (Method::GET, "/v1/api-keys", Permission::ReadApiKey, None),
        (
            Method::POST,
            "/v1/api-keys",
            Permission::WriteApiKey,
            body(CreateApiKeyRequest {
                user_id: None,
                name: unique("guard"),
                permissions: vec![],
                description: None,
                expiry: None,
            }),
        ),
        (Method::GET, "/v1/lightning-addresses", Permission::ReadLnAddress, None),
        (
            Method::POST,
            "/v1/lightning-addresses",
            Permission::WriteLnAddress,
            body(RegisterLnAddressRequest {
                wallet_id: None,
                username: unique("guard"),
                allows_nostr: false,
                nostr_pubkey: None,
            }),
        ),
        (Method::GET, "/v1/bitcoin/addresses", Permission::ReadBtcAddress, None),
        (
            Method::POST,
            "/v1/bitcoin/addresses",
            Permission::WriteBtcAddress,
            body(NewBtcAddressRequest {
                wallet_id: None,
                address_type: None,
            }),
        ),
    ];

    for (method, path, required, payload) in cases {
        let scopes: Vec<Permission> = Permission::all_permissions()
            .into_iter()
            .filter(|p| p != &required)
            .collect();
        let key = app.api_key(token, scopes).await;

        let res = app
            .api()
            .request(method.clone(), path, Auth::ApiKey(&key), payload)
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
