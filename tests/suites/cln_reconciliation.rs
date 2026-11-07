//! Exercise production adapters/listeners and durable accounting through HTTP.
//! A wire-level CLN peer makes xpay retry and MPP timing deterministic.
use std::{sync::atomic::Ordering, time::Duration};

use reqwest::StatusCode;
use swissknife_types::{Payment, PaymentFeeEstimate, PaymentStatus, SendPaymentRequest};
use uuid::Uuid;

use crate::common::{
    assert_error, assert_status,
    cln_scenario::{invoice, Outcome, Peer, AMOUNT, FINAL_FEE, RESERVED},
    harness::matrix_cell,
    wait::wait_until,
    Auth, TestApp,
};

fn request(wallet: Uuid, seed: u8) -> SendPaymentRequest {
    SendPaymentRequest {
        wallet_id: Some(wallet),
        input: invoice(seed, true),
        amount_msat: None,
        comment: None,
    }
}

async fn setup(label: &str) -> Option<(TestApp, Peer, Uuid)> {
    let (_, provider) = matrix_cell();
    if !provider.starts_with("cln_") {
        return None;
    }
    let funded = TestApp::isolated(label, &[]).await;
    let token = funded.admin_token().await;
    let wallet = funded.create_wallet(token, label).await;
    funded.fund_onchain(token, wallet.id, RESERVED / 1000).await;
    let peer = Peer::start(&provider).await;
    let app = funded.restart(&format!("{label}-peer"), &peer.env).await;
    Some((app, peer, wallet.id))
}

async fn payment(app: &TestApp, wallet: Uuid) -> Payment {
    let res = app
        .api()
        .get(
            &format!("/v1/payments?wallet_id={wallet}"),
            Auth::Bearer(app.admin_token().await),
        )
        .await;
    assert_status(&res, StatusCode::OK);
    let payments = res.parse::<Vec<Payment>>();
    assert_eq!(payments.len(), 1, "one durable send, including after retries/restart");
    payments.into_iter().next().unwrap()
}

async fn start_send(app: &TestApp, peer: &Peer, wallet: Uuid) {
    let res = app
        .api()
        .post(
            "/v1/payments",
            Auth::Bearer(app.admin_token().await),
            request(wallet, 21),
        )
        .await;
    assert_status(&res, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(peer.scenario.sends.load(Ordering::SeqCst), 1);
    assert_held(app, wallet).await;
}

async fn assert_held(app: &TestApp, wallet: Uuid) {
    assert_eq!(payment(app, wallet).await.status, PaymentStatus::Pending);
    let balance = app.wallet_balance(app.admin_token().await, wallet).await;
    assert_eq!(balance.available_msat, 0, "an unresolved send cannot be spent again");
    assert_eq!(balance.reserved_msat, RESERVED);
}

async fn reconcile_event(peer: &Peer, status: &str) {
    let before = peer.scenario.lookups.load(Ordering::SeqCst);
    peer.scenario.notify(status);
    wait_until(
        Duration::from_secs(10),
        "event or periodic aggregate lookup",
        || async { peer.scenario.lookups.load(Ordering::SeqCst) > before },
    )
    .await;
    // Allow the lookup's projection to commit before asserting its effect.
    tokio::time::sleep(Duration::from_millis(200)).await;
}

async fn assert_settled(app: &TestApp, wallet: Uuid) {
    wait_until(Duration::from_secs(15), "whole payment settled", || async {
        payment(app, wallet).await.status == PaymentStatus::Settled
    })
    .await;
    let pay = payment(app, wallet).await;
    assert_eq!(pay.amount_msat, AMOUNT);
    assert_eq!(pay.fee_msat, Some(FINAL_FEE));
    let balance = app.wallet_balance(app.admin_token().await, wallet).await;
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(balance.available_msat, (RESERVED - AMOUNT - FINAL_FEE) as i64);
}

#[tokio::test]
async fn failed_attempt_cannot_release_funds_for_a_second_spend() {
    let Some((app, peer, wallet)) = setup("cln-retry-hold").await else {
        return;
    };
    start_send(&app, &peer, wallet).await;
    // Real CLN versions can report either stale `pending` or `failed` in this
    // per-attempt notification. Neither is a whole-xpay terminal outcome.
    for status in ["pending", "failed"] {
        reconcile_event(&peer, status).await;
        assert_held(&app, wallet).await;
        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(app.admin_token().await),
                request(wallet, 22),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(res.body["reason"].as_str().unwrap().contains("Insufficient funds"));
        assert_eq!(peer.scenario.sends.load(Ordering::SeqCst), 1);
    }
    peer.scenario.set(Outcome::Complete);
    reconcile_event(&peer, "complete").await;
    assert_settled(&app, wallet).await;
    // Replayed success/failure cannot debit twice or downgrade a settlement.
    peer.scenario.notify("failed");
    peer.scenario.notify("complete");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_settled(&app, wallet).await;
}

#[tokio::test]
async fn partial_success_keeps_the_reservation_until_all_parts_resolve() {
    let Some((app, peer, wallet)) = setup("cln-mpp-hold").await else {
        return;
    };
    start_send(&app, &peer, wallet).await;
    for outcome in [Outcome::Partial, Outcome::Incomplete] {
        peer.scenario.set(outcome);
        reconcile_event(&peer, "complete").await;
        assert_held(&app, wallet).await;
    }
    // Deliberately no final notification: periodic reconciliation must finish.
    peer.scenario.set(Outcome::Complete);
    assert_settled(&app, wallet).await;
}

#[tokio::test]
async fn restart_and_lookup_uncertainty_preserve_the_hold_until_terminal_failure() {
    let Some((app, peer, wallet)) = setup("cln-recovery-hold").await else {
        return;
    };
    start_send(&app, &peer, wallet).await;
    peer.scenario.set(Outcome::LookupError);
    reconcile_event(&peer, "failed").await;
    assert_held(&app, wallet).await;
    peer.scenario.set(Outcome::Missing);
    let restarted = app.restart("cln-recovery-restarted", &peer.env).await;
    assert_held(&restarted, wallet).await;
    assert_eq!(
        peer.scenario.sends.load(Ordering::SeqCst),
        1,
        "recovery must never resend"
    );
    peer.scenario.set(Outcome::Failed);
    wait_until(
        Duration::from_secs(15),
        "terminal failure reconciled without notification",
        || async { payment(&restarted, wallet).await.status == PaymentStatus::Failed },
    )
    .await;
    let balance = restarted.wallet_balance(restarted.admin_token().await, wallet).await;
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(balance.available_msat, RESERVED as i64);
}

#[tokio::test]
async fn private_hint_estimate_includes_suffix_fees_and_cltv_without_sending() {
    let Some((app, peer, wallet)) = setup("cln-private-quote").await else {
        return;
    };
    let res = app
        .api()
        .post(
            "/v1/payments/fee-estimate",
            Auth::Bearer(app.admin_token().await),
            request(wallet, 21),
        )
        .await;
    assert_status(&res, StatusCode::OK);
    let estimate = res.parse::<PaymentFeeEstimate>();
    assert_eq!(estimate.estimated_fee_msat, Some(4_734));
    assert_eq!(estimate.maximum_fee_msat, 10_000);
    assert_eq!(peer.scenario.sends.load(Ordering::SeqCst), 0);
    let quotes = peer.scenario.quotes.lock().unwrap();
    let hinted = quotes
        .iter()
        .find(|quote| quote["amount_msat"] == 1_003_500)
        .expect("quote the public entry to the private route");
    assert_eq!(hinted["final_cltv"], 184);
    assert_eq!(hinted["maxfee_msat"], 6_500);
    assert_eq!(hinted["maxparts"], 1);
}

#[tokio::test]
async fn an_early_xpay_success_response_cannot_release_pending_mpp_fees() {
    for (label, outcome) in [
        ("cln-rpc-mpp-hold", Outcome::Partial),
        ("cln-rpc-partial-value", Outcome::Incomplete),
    ] {
        let Some((app, peer, wallet)) = setup(label).await else {
            return;
        };
        peer.scenario.set(outcome);
        peer.scenario.rpc_success.store(true, Ordering::SeqCst);
        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(app.admin_token().await),
                request(wallet, 21),
            )
            .await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(res.parse::<Payment>().status, PaymentStatus::Pending);
        assert_held(&app, wallet).await;
        peer.scenario.set(Outcome::Complete);
        assert_settled(&app, wallet).await;
        app.stop();
    }
}
