//! Real CLN attempt notifications must not release an in-flight payment's funds.
use std::time::Duration;

use reqwest::StatusCode;
use swissknife_types::{ClientEventType, Payment, PaymentStatus, SendPaymentRequest};
use tokio::time::sleep;
use uuid::Uuid;

use super::events::next_event;
use crate::common::{
    assert_error, assert_status, counterparty::Counterparty, harness::matrix_cell, wait::wait_until, Auth, TestApp,
};

const AMOUNT: u64 = 1_000_000;
const RESERVED: u64 = AMOUNT + 10_000;

fn request(wallet: Uuid, invoice: &str) -> SendPaymentRequest {
    SendPaymentRequest {
        wallet_id: Some(wallet),
        input: invoice.to_string(),
        amount_msat: None,
        comment: None,
    }
}

async fn setup(label: &str, timeout: &str) -> Option<(TestApp, Uuid, String)> {
    if !matrix_cell().1.starts_with("cln_") {
        return None;
    }
    let app = TestApp::isolated(
        label,
        &[
            ("SWISSKNIFE_WEB__REQUEST_TIMEOUT", timeout.to_string()),
            ("SWISSKNIFE_CLN_REST_CONFIG__TIMEOUT", "3s".to_string()),
        ],
    )
    .await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, label).await;
    app.fund_onchain(token, wallet.id, RESERVED / 1000).await;
    let key = app.account_api_key(token, wallet.account_id, vec![]).await;
    Some((app, wallet.id, key))
}

async fn payment(app: &TestApp, wallet: Uuid) -> Payment {
    let response = app
        .api()
        .get(
            &format!("/v1/payments?wallet_id={wallet}"),
            Auth::Bearer(app.admin_token().await),
        )
        .await;
    assert_status(&response, StatusCode::OK);
    let mut payments = response.parse::<Vec<Payment>>();
    assert_eq!(payments.len(), 1, "one original payment, no resend");
    payments.remove(0)
}

async fn assert_reserved(app: &TestApp, wallet: Uuid) {
    assert_eq!(payment(app, wallet).await.status, PaymentStatus::Pending);
    let balance = app.wallet_balance(app.admin_token().await, wallet).await;
    assert_eq!(balance.available_msat, 0);
    assert_eq!(balance.reserved_msat, RESERVED);
}

#[tokio::test]
async fn failed_part_cannot_release_funds_for_a_second_spend() {
    let Some((app, wallet, key)) = setup("cln-failed-part", "30s").await else {
        return;
    };
    let mut held = Counterparty::for_provider(&app.provider).hold_invoice(AMOUNT);
    let api = app.api();
    let token = app.admin_token().await.to_string();
    let send = request(wallet, &held.bolt11);
    let sending = tokio::spawn(async move { api.post("/v1/payments", Auth::Bearer(&token), send).await });
    wait_until(
        Duration::from_secs(10),
        "receiver accepted the original payment",
        || async { held.accepted() },
    )
    .await;
    let mut events = app.api().event_stream("/v1/me/events", Auth::ApiKey(&key), None).await;

    // This emits an actual sendpay_failure / wait(sendpays) update. It does not
    // fake CLN responses: listpays still reports the held payment as pending.
    Counterparty::for_provider("lnd_grpc").fail_payment_part(&held.hash);
    sleep(Duration::from_secs(1)).await;
    assert_reserved(&app, wallet).await;
    let second_invoice = Counterparty::for_provider(&app.provider).invoice(AMOUNT, "second-spend");
    let second = app
        .api()
        .post(
            "/v1/payments",
            Auth::Bearer(app.admin_token().await),
            request(wallet, &second_invoice),
        )
        .await;
    assert_error(&second, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(second.body["reason"].as_str().unwrap().contains("Insufficient funds"));

    held.settle();
    let response = sending.await.unwrap();
    assert_status(&response, StatusCode::OK);
    assert_eq!(response.parse::<Payment>().status, PaymentStatus::Settled);
    let event = next_event(&mut events).await;
    assert_eq!(
        event.payload.event_type,
        ClientEventType::PaymentSettled,
        "no false PaymentFailed event"
    );
    assert_eq!(event.payload.resource_id, payment(&app, wallet).await.id);
    let balance = app.wallet_balance(app.admin_token().await, wallet).await;
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(
        balance.available_msat, 10_000,
        "one debit, with zero fees on the direct channel"
    );
}

#[tokio::test]
async fn completes_a_payment_after_the_http_request_times_out() {
    for settle in [true, false] {
        let label = if settle { "cln-late-success" } else { "cln-late-failure" };
        let Some((app, wallet, key)) = setup(label, "2s").await else {
            return;
        };
        let mut held = Counterparty::for_provider(&app.provider).hold_invoice(AMOUNT);
        let response = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(app.admin_token().await),
                request(wallet, &held.bolt11),
            )
            .await;
        assert_status(&response, StatusCode::REQUEST_TIMEOUT);
        assert!(held.accepted());
        assert_reserved(&app, wallet).await;
        // Also exceed the ordinary REST RPC deadline: an accepted HTLC can
        // outlive both that deadline and the caller's HTTP request.
        sleep(Duration::from_secs(2)).await;
        let mut events = app.api().event_stream("/v1/me/events", Auth::ApiKey(&key), None).await;
        if settle {
            held.settle();
        } else {
            held.cancel();
        }
        let expected = if settle {
            PaymentStatus::Settled
        } else {
            PaymentStatus::Failed
        };
        wait_until(Duration::from_secs(10), "final notification committed", || async {
            payment(&app, wallet).await.status == expected
        })
        .await;
        let event = next_event(&mut events).await;
        assert_eq!(
            event.payload.event_type,
            if settle {
                ClientEventType::PaymentSettled
            } else {
                ClientEventType::PaymentFailed
            }
        );
        let balance = app.wallet_balance(app.admin_token().await, wallet).await;
        assert_eq!(balance.reserved_msat, 0);
        assert_eq!(balance.available_msat, if settle { 10_000 } else { RESERVED as i64 });
        app.stop();
    }
}

#[tokio::test]
async fn startup_recovers_a_payment_settled_while_swissknife_was_stopped() {
    let Some((app, wallet, _key)) = setup("cln-restart", "2s").await else {
        return;
    };
    let mut held = Counterparty::for_provider(&app.provider).hold_invoice(AMOUNT);
    let response = app
        .api()
        .post(
            "/v1/payments",
            Auth::Bearer(app.admin_token().await),
            request(wallet, &held.bolt11),
        )
        .await;
    assert_status(&response, StatusCode::REQUEST_TIMEOUT);
    assert!(held.accepted());
    assert_reserved(&app, wallet).await;
    app.stop();
    held.settle();
    let restarted = app.restart("cln-recovered", &[]).await;
    assert_eq!(payment(&restarted, wallet).await.status, PaymentStatus::Settled);
    let balance = restarted.wallet_balance(restarted.admin_token().await, wallet).await;
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(balance.available_msat, 10_000);
}
