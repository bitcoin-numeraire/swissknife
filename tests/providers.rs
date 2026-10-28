//! Lightning-only provider contract. LDK does not expose the PSBT/output RPCs
//! required by the on-chain suite; unsupported operations are asserted here.
//! Auth, account and invoice suites remain shared with the existing adapters.
#![cfg(feature = "itest")]

#[path = "suites/accounts.rs"]
mod accounts;
#[path = "suites/api_keys.rs"]
mod api_keys;
#[path = "suites/auth.rs"]
mod auth;
mod common;
#[path = "suites/guards.rs"]
mod guards;
#[path = "suites/invoices.rs"]
mod invoices;
#[path = "suites/ln_addresses.rs"]
mod ln_addresses;
#[path = "suites/system.rs"]
mod system;
#[path = "suites/well_known.rs"]
mod well_known;

use common::{app, assert_status, counterparty::Counterparty, fixtures::unique, wait::wait_until, Auth, TestApp};
use reqwest::StatusCode;
use std::time::Duration;
use swissknife_types::{
    Invoice, InvoiceStatus, NewBtcAddressRequest, NewInvoiceRequest, Payment, PaymentFeeEstimate, PaymentStatus,
    SendPaymentRequest,
};

async fn invoice(app: &TestApp, wallet_id: uuid::Uuid, amount_msat: u64) -> Invoice {
    let response = app
        .api()
        .post(
            "/v1/invoices",
            Auth::Bearer(app.admin_token().await),
            NewInvoiceRequest {
                wallet_id: Some(wallet_id),
                amount_msat,
                description: Some("LDK integration".into()),
                expiry: Some(3600),
            },
        )
        .await;
    assert_status(&response, StatusCode::OK);
    response.parse()
}

async fn fund(app: &TestApp, wallet_id: uuid::Uuid, amount_msat: u64) {
    let token = app.admin_token().await;
    let before = app.wallet_balance(token, wallet_id).await.available_msat;
    let incoming = invoice(app, wallet_id, amount_msat).await;
    Counterparty::for_provider(&app.provider).pay(&incoming.ln_invoice.unwrap().bolt11);
    wait_until(Duration::from_secs(30), "LDK incoming payment settled", || async {
        app.wallet_balance(token, wallet_id).await.available_msat == before + amount_msat as i64
    })
    .await;
}

#[tokio::test]
async fn receive_preserves_millisatoshis_expiry_and_replays_exactly_once() {
    let app = TestApp::isolated("ldk-offline-receive", &[]).await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "offline-receive").await;
    let amount = 100_000_123;
    let incoming = invoice(&app, wallet.id, amount).await;
    let ln = incoming.ln_invoice.as_ref().unwrap();
    assert_eq!(incoming.amount_msat, Some(amount));
    assert_eq!(ln.expiry, Duration::from_secs(3600));
    app.stop();
    Counterparty::for_provider(&app.provider).pay(&ln.bolt11);
    let restarted = app.restart("ldk-offline-receive-restarted", &[]).await;
    wait_until(Duration::from_secs(30), "offline receive replayed", || async {
        restarted.wallet_balance(token, wallet.id).await.available_msat == amount as i64
    })
    .await;
    let again = restarted.restart("ldk-offline-receive-twice", &[]).await;
    assert_eq!(
        again.wallet_balance(token, wallet.id).await.available_msat,
        amount as i64
    );
    let persisted = again
        .api()
        .get(&format!("/v1/invoices/{}", incoming.id), Auth::Bearer(token))
        .await
        .parse::<Invoice>();
    assert_eq!(persisted.status, InvoiceStatus::Settled);
    assert_eq!(persisted.amount_received_msat, Some(amount));
}

#[tokio::test]
async fn sends_with_actual_fee_and_proof_while_quote_exposes_only_a_cap() {
    let app = app().await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "ldk-send").await;
    fund(app, wallet.id, 100_000_000).await;
    let amount = 21_123;
    let preimage = random_preimage();
    let bolt11 = Counterparty::for_provider("lnd_grpc").invoice_with_preimage(amount, &unique("ldk-out"), &preimage);
    let request = SendPaymentRequest {
        wallet_id: Some(wallet.id),
        input: bolt11,
        amount_msat: None,
        comment: None,
    };
    let quote = app
        .api()
        .post("/v1/payments/fee-estimate", Auth::Bearer(token), request.clone())
        .await;
    assert_status(&quote, StatusCode::OK);
    let quote = quote.parse::<PaymentFeeEstimate>();
    assert!(quote.estimated_fee_msat.is_none());
    assert!(quote.estimated_total_msat.is_none());
    assert_eq!(quote.maximum_fee_msat, 25_000);
    let response = app.api().post("/v1/payments", Auth::Bearer(token), request).await;
    assert_status(&response, StatusCode::OK);
    let payment = response.parse::<Payment>();
    assert_eq!(payment.status, PaymentStatus::Settled);
    assert_eq!(payment.amount_msat, amount);
    assert_eq!(payment.lightning.unwrap().payment_preimage, Some(hex::encode(preimage)));
    let fee = payment.fee_msat.expect("actual fee recorded");
    assert!(fee > 0 && fee <= quote.maximum_fee_msat);
    let balance = app.wallet_balance(token, wallet.id).await;
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(balance.available_msat, (100_000_000 - amount - fee) as i64);
}

#[tokio::test]
async fn unsupported_onchain_operations_fail_before_reservation() {
    let app = app().await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "ldk-onchain-unavailable").await;
    let response = app
        .api()
        .post(
            "/v1/bitcoin/addresses",
            Auth::Bearer(token),
            NewBtcAddressRequest {
                wallet_id: Some(wallet.id),
                address_type: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.body["reason"].as_str().unwrap().contains("LDK Server"));
    let destination = common::chain::new_address().await;
    let response = app
        .api()
        .post(
            "/v1/payments",
            Auth::Bearer(token),
            SendPaymentRequest {
                wallet_id: Some(wallet.id),
                input: destination,
                amount_msat: Some(100_000),
                comment: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.wallet_balance(token, wallet.id).await.reserved_msat, 0);
}

#[tokio::test]
async fn pays_an_external_lnurl_with_an_authenticated_settlement_proof() {
    let app = app().await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "ldk-lnurl-send").await;
    fund(app, wallet.id, 100_000_000).await;
    let preimage = random_preimage();
    let amount = 51_234;
    let bolt11 =
        Counterparty::for_provider(&app.provider).invoice_with_preimage(amount, &unique("ldk-lnurl"), &preimage);
    let mock = common::MockLnurl::start().await;
    let user = unique("payee");
    mock.mount_pay_request(&user, 1000, 100_000_000, 0).await;
    mock.mount_callback_invoice(&bolt11).await;
    let response = app
        .api()
        .post(
            "/v1/payments",
            Auth::Bearer(token),
            SendPaymentRequest {
                wallet_id: Some(wallet.id),
                input: mock.lnurlp_url(&user),
                amount_msat: Some(amount),
                comment: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::OK);
    let payment = response.parse::<Payment>();
    assert_eq!(payment.status, PaymentStatus::Settled);
    assert_eq!(payment.amount_msat, amount);
    assert_eq!(payment.lightning.unwrap().payment_preimage, Some(hex::encode(preimage)));
}

fn random_preimage() -> [u8; 32] {
    let mut result = [0; 32];
    result[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    result[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    result
}

#[tokio::test]
async fn a_zero_fee_cap_does_not_authorize_a_routed_payment() {
    let app = TestApp::isolated(
        "ldk-zero-cap",
        &[("SWISSKNIFE_LDK_SERVER_CONFIG__FEE_LIMIT_MSAT", "0".into())],
    )
    .await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "zero-cap").await;
    fund(&app, wallet.id, 100_000_000).await;
    // LDK -> LND -> CLN necessarily pays the middle node's forwarding fee.
    let bolt11 = Counterparty::for_provider("lnd_grpc").invoice(100_000, &unique("ldk-zero-fee"));
    let response = app
        .api()
        .post(
            "/v1/payments",
            Auth::Bearer(token),
            SendPaymentRequest {
                wallet_id: Some(wallet.id),
                input: bolt11,
                amount_msat: None,
                comment: None,
            },
        )
        .await;
    assert!(matches!(
        response.status,
        StatusCode::OK | StatusCode::UNPROCESSABLE_ENTITY
    ));
    // An explicit failure releases the reservation; an ambiguous reply keeps
    // it pending. Neither may be reported/debited as a completed payment.
    let payments = app
        .api()
        .get(&format!("/v1/payments?wallet_id={}", wallet.id), Auth::Bearer(token))
        .await
        .parse::<Vec<Payment>>();
    assert_eq!(payments.len(), 1);
    assert_ne!(payments[0].status, PaymentStatus::Settled);
    let balance = app.wallet_balance(token, wallet.id).await;
    assert_eq!(balance.available_msat + balance.reserved_msat as i64, 100_000_000);
}
