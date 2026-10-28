//! Explicit opt-in, real public waved + public signet/testnet services through
//! SwissKnife's HTTP API. No funds move and no funded coverage is claimed.
#![cfg(feature = "public-itest")]
#[allow(unused_imports)]
mod common;

use common::{assert_status, fixtures::unique, Auth, TestApp};
use reqwest::StatusCode;
use std::path::PathBuf;
use swissknife_types::{Invoice, InvoiceStatus, NewInvoiceRequest, Wallet};

#[tokio::test]
async fn real_public_wallet_creates_a_receive_through_swissknife_and_preserves_it_on_restart() {
    let network = std::env::var("WAVELENGTH_PUBLIC_NETWORK").expect("public test network");
    assert!(matches!(network.as_str(), "signet" | "testnet"));
    let credentials = PathBuf::from(std::env::var("WAVELENGTH_TEST_CREDENTIAL_DIR").expect("RPC credential directory"));
    let env = vec![
        ("SWISSKNIFE_INVOICE_EXPIRY", "provider".into()),
        (
            "SWISSKNIFE_WAVELENGTH_CONFIG__ENDPOINT",
            std::env::var("WAVELENGTH_TEST_ENDPOINT").unwrap(),
        ),
        (
            "SWISSKNIFE_WAVELENGTH_CONFIG__CERT_PATH",
            credentials.join("tls.cert").display().to_string(),
        ),
        (
            "SWISSKNIFE_WAVELENGTH_CONFIG__MACAROON_PATH",
            credentials.join("admin.macaroon").display().to_string(),
        ),
        ("SWISSKNIFE_WAVELENGTH_CONFIG__ALLOW_SERVER_CREDIT", "true".into()),
    ];
    let app = TestApp::isolated_for("sqlite", "wavelength", &unique("wave-public-api"), &env).await;
    let token = app.admin_token().await;
    let wallets = app
        .api()
        .get("/v1/me/wallets", Auth::Bearer(token))
        .await
        .parse::<Vec<Wallet>>();
    assert_eq!(wallets.len(), 1);
    let wallet = &wallets[0];
    assert_eq!(wallet.balance.available_msat, 0);
    assert_eq!(
        wallet.asset.as_ref().unwrap().network,
        if network == "signet" {
            swissknife_types::BtcNetwork::Signet
        } else {
            swissknife_types::BtcNetwork::Testnet
        }
    );
    let response = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/invoices", wallet.id),
            Auth::Bearer(token),
            NewInvoiceRequest {
                wallet_id: None,
                amount_msat: 21_000_000,
                description: Some("SwissKnife public interoperability".into()),
                expiry: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::OK);
    let invoice = response.parse::<Invoice>();
    assert_eq!(invoice.status, InvoiceStatus::Pending);
    assert_eq!(invoice.amount_msat, Some(21_000_000));
    let details = invoice.ln_invoice.as_ref().unwrap();
    assert!(details
        .bolt11
        .starts_with(if network == "signet" { "lntbs" } else { "lntb" }));
    assert!(details.expiry.as_secs() > 0);
    let response = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/invoices", wallet.id),
            Auth::Bearer(token),
            NewInvoiceRequest {
                wallet_id: None,
                amount_msat: 21_000_000,
                description: None,
                expiry: Some(30),
            },
        )
        .await;
    assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    let restarted = app.restart(&unique("wave-public-api-restart"), &env).await;
    let response = restarted
        .api()
        .get(&format!("/v1/invoices/{}", invoice.id), Auth::Bearer(token))
        .await;
    assert_status(&response, StatusCode::OK);
    let persisted = response.parse::<Invoice>();
    assert_eq!(persisted.ln_invoice.unwrap().payment_hash, details.payment_hash);
    assert_eq!(persisted.status, InvoiceStatus::Pending);
    assert_eq!(restarted.wallet_balance(token, wallet.id).await.available_msat, 0);
    restarted.shutdown();
}
