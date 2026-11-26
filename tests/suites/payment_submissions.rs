//! Exercise the public admission and status resources across real node transports.
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::json;
use swissknife_types::{Invoice, PaymentExecutionState, PaymentSubmission};
use uuid::Uuid;

use crate::common::{
    assert_error, assert_status, counterparty::Counterparty, fixtures::unique, wait::wait_until, Auth, MockLnurl,
    TestApp,
};

#[tokio::test]
async fn committed_submission_is_idempotent_owned_and_survives_restart() {
    let app = TestApp::isolated("durable-ln-submit", &[]).await;
    let admin = app.admin_token().await;
    let alice = app.create_account_with_wallet(admin, "submit-alice").await;
    let bob = app.create_account_with_wallet(admin, "submit-bob").await;
    app.fund_onchain(admin, alice.wallet.id, 100_000).await;
    let before = app.wallet_balance(admin, alice.wallet.id).await;
    let path = format!("/v1/me/wallets/{}/payments/submissions", alice.wallet.id);
    let invoice = Counterparty::for_provider(&app.provider).invoice(42_000, &unique("durable-submit"));
    let mut req = json!({"idempotency_key":Uuid::new_v4(),"input":invoice,"maximum_fee_msat":5_000});
    let api = app.api();
    assert_error(&api.post(&path, Auth::None, &req).await, StatusCode::UNAUTHORIZED);
    let foreign = api.post(&path, Auth::ApiKey(&bob.key), &req).await;
    assert!(
        matches!(foreign.status, StatusCode::NOT_FOUND | StatusCode::FORBIDDEN),
        "foreign submit: {}",
        foreign.status
    );
    let (a, b) = tokio::join!(
        api.post(&path, Auth::ApiKey(&alice.key), &req),
        api.post(&path, Auth::ApiKey(&alice.key), &req)
    );
    assert_status(&a, StatusCode::ACCEPTED);
    assert_status(&b, StatusCode::ACCEPTED);
    let a: PaymentSubmission = a.parse();
    let b: PaymentSubmission = b.parse();
    assert_eq!(a.payment.id, b.payment.id);
    assert_eq!(a.status_url, format!("{path}/{}", a.payment.id));
    let other_path = format!("/v1/me/wallets/{}/payments/submissions/{}", bob.wallet.id, a.payment.id);
    assert_error(
        &api.get(&other_path, Auth::ApiKey(&bob.key)).await,
        StatusCode::NOT_FOUND,
    );
    wait_until(
        Duration::from_secs(45),
        "worker settles the accepted invoice",
        || async {
            api.get(&a.status_url, Auth::ApiKey(&alice.key))
                .await
                .parse::<PaymentSubmission>()
                .state
                == PaymentExecutionState::Settled
        },
    )
    .await;
    let final_payment = api
        .get(&a.status_url, Auth::ApiKey(&alice.key))
        .await
        .parse::<PaymentSubmission>()
        .payment;
    let after = app.wallet_balance(admin, alice.wallet.id).await;
    assert_eq!(after.reserved_msat, 0);
    assert_eq!(
        before.available_msat - after.available_msat,
        (42_000 + final_payment.fee_msat.unwrap_or_default()) as i64
    );
    req["maximum_fee_msat"] = json!(4_999);
    assert_error(
        &api.post(&path, Auth::ApiKey(&alice.key), &req).await,
        StatusCode::CONFLICT,
    );
    req["maximum_fee_msat"] = json!(5_000);
    let restarted = app.restart("replayed-durable-submit", &[]).await;
    let replay = restarted.api().post(&path, Auth::ApiKey(&alice.key), &req).await;
    assert_status(&replay, StatusCode::ACCEPTED);
    let replay: PaymentSubmission = replay.parse();
    assert_eq!(replay.state, PaymentExecutionState::Settled);
    assert_eq!(replay.payment.id, a.payment.id);
    assert_eq!(
        restarted.wallet_balance(admin, alice.wallet.id).await.available_msat,
        after.available_msat
    );
}

#[tokio::test]
async fn local_or_invalid_destinations_are_rejected_before_reservation() {
    let app = TestApp::isolated("command-validation", &[]).await;
    let admin = app.admin_token().await;
    let wallet = app.create_wallet(admin, "command-validation").await;
    let receiver = app.create_wallet(admin, "command-receiver").await;
    let api = app.api();
    let invoice = api
        .post(
            "/v1/invoices",
            Auth::Bearer(admin),
            json!({
                "wallet_id": receiver.id, "amount_msat": 15_000, "description": "local"
            }),
        )
        .await;
    assert_status(&invoice, StatusCode::OK);
    let input = invoice.parse::<Invoice>().ln_invoice.unwrap().bolt11;
    let path = "/v1/payments/submissions";
    let mut req = json!({"wallet_id":wallet.id,"idempotency_key":Uuid::new_v4(),"input":input});
    let local = api.post(path, Auth::Bearer(admin), &req).await;
    assert_error(&local, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(local.body["code"], "internal_transfer_required");
    req["input"] = json!("invalid");
    let invalid = api.post(path, Auth::Bearer(admin), &req).await;
    assert_error(&invalid, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(invalid.body.get("code").is_none());
    assert_eq!(app.wallet_balance(admin, wallet.id).await.reserved_msat, 0);
}

#[tokio::test]
async fn accepted_held_payment_completes_after_client_and_server_restart() {
    if !crate::common::harness::matrix_cell().1.starts_with("cln_") {
        return;
    }
    let app = TestApp::isolated("command-held", &[]).await;
    let admin = app.admin_token().await;
    let alice = app.create_account_with_wallet(admin, "held-commands").await;
    app.fund_onchain(admin, alice.wallet.id, 100_000).await;
    let before = app.wallet_balance(admin, alice.wallet.id).await;
    let mut held = Counterparty::for_provider(&app.provider).hold_invoice(500_000);
    let req = json!({"wallet_id":alice.wallet.id,"idempotency_key":Uuid::new_v4(),"input":held.bolt11,"maximum_fee_msat":1_000});
    // Admission returns before the recipient releases the HTLC.
    let response = app
        .api()
        .post("/v1/payments/submissions", Auth::Bearer(admin), &req)
        .await;
    assert_status(&response, StatusCode::ACCEPTED);
    let a = response.parse::<PaymentSubmission>();
    assert_eq!(a.state, PaymentExecutionState::Accepted);
    wait_until(Duration::from_secs(30), "receiver holds accepted payment", || async {
        held.accepted()
    })
    .await;
    let processing = app
        .api()
        .get(&a.status_url, Auth::Bearer(admin))
        .await
        .parse::<PaymentSubmission>();
    assert_eq!(processing.state, PaymentExecutionState::Processing);
    assert_eq!(app.wallet_balance(admin, alice.wallet.id).await.reserved_msat, 501_000);
    app.stop();
    held.settle();
    let restarted = app.restart("command-held-reconciled", &[]).await;
    wait_until(
        Duration::from_secs(45),
        "restart reconciles a dispatched payment",
        || async {
            restarted
                .api()
                .get(&a.status_url, Auth::Bearer(admin))
                .await
                .parse::<PaymentSubmission>()
                .state
                == PaymentExecutionState::Settled
        },
    )
    .await;
    let final_balance = restarted.wallet_balance(admin, alice.wallet.id).await;
    assert_eq!(final_balance.reserved_msat, 0);
    assert_eq!(final_balance.available_msat, before.available_msat - 500_000);
    let receipt = restarted
        .api()
        .post("/v1/payments/submissions", Auth::Bearer(admin), &req)
        .await
        .parse::<PaymentSubmission>();
    assert_eq!(receipt.payment.id, a.payment.id);
    assert_eq!(receipt.state, PaymentExecutionState::Settled);
}

#[tokio::test]
async fn cancelled_hold_invoice_releases_the_reservation_without_restarting_or_resending() {
    if !crate::common::harness::matrix_cell().1.starts_with("cln_") {
        return;
    }
    let app = TestApp::isolated("command-cancelled-htlc", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "cancelled-command").await;
    app.fund_onchain(admin, owner.wallet.id, 10_000).await;
    let before = app.wallet_balance(admin, owner.wallet.id).await;
    let held = Counterparty::for_provider(&app.provider).hold_invoice(500_000);
    let request = json!({"idempotency_key":Uuid::new_v4(),"input":held.bolt11,"maximum_fee_msat":1000});
    let path = format!("/v1/me/wallets/{}/payments/submissions", owner.wallet.id);
    let auth = Auth::ApiKey(&owner.key);
    let api = app.api();
    let response = api.post(&path, auth, &request).await;
    assert_status(&response, StatusCode::ACCEPTED);
    let receipt = response.parse::<PaymentSubmission>();
    wait_until(Duration::from_secs(30), "receiver holds the payment", || async {
        held.accepted()
    })
    .await;
    assert_eq!(app.wallet_balance(admin, owner.wallet.id).await.reserved_msat, 501_000);

    // Dropping a held invoice asks the real recipient to cancel the HTLC.
    drop(held);
    wait_until(
        Duration::from_secs(75),
        "confirmed cancellation releases funds",
        || async {
            api.get(&receipt.status_url, auth)
                .await
                .parse::<PaymentSubmission>()
                .state
                == PaymentExecutionState::Failed
        },
    )
    .await;
    let replay = api.post(&path, auth, &request).await.parse::<PaymentSubmission>();
    assert_eq!(replay.payment.id, receipt.payment.id);
    assert_eq!(replay.state, PaymentExecutionState::Failed);
    let after = app.wallet_balance(admin, owner.wallet.id).await;
    assert_eq!(
        (after.available_msat, after.reserved_msat),
        (before.available_msat, before.reserved_msat)
    );
    app.stop();
}

#[tokio::test]
async fn an_unroutable_submission_preserves_the_provider_outcome_and_wallet_invariant() {
    let app = TestApp::isolated("command-unroutable", &[]).await;
    let admin = app.admin_token().await;
    let wallet = app.create_wallet(admin, "unroutable-command").await;
    app.fund_onchain(admin, wallet.id, 10_000_000).await;
    let before = app.wallet_balance(admin, wallet.id).await;
    let amount = 8_000_000_000u64;
    let invoice = Counterparty::for_provider(&app.provider).invoice(amount, &unique("unroutable-command"));
    let request =
        json!({"idempotency_key":Uuid::new_v4(),"wallet_id":wallet.id,"input":invoice,"maximum_fee_msat":1_000});
    let r = app
        .api()
        .post("/v1/payments/submissions", Auth::Bearer(admin), &request)
        .await;
    assert_status(&r, StatusCode::ACCEPTED);
    let receipt = r.parse::<PaymentSubmission>();

    let ambiguous = app.provider.starts_with("cln_");
    let expected = if ambiguous {
        PaymentExecutionState::Processing
    } else {
        PaymentExecutionState::Failed
    };
    wait_until(
        Duration::from_secs(60),
        "provider completes its unroutable attempt",
        || async {
            if ambiguous && !app.logs_contain("Payment outcome unknown; preserving reservation") {
                return false;
            }
            app.api()
                .get(&receipt.status_url, Auth::Bearer(admin))
                .await
                .parse::<PaymentSubmission>()
                .state
                == expected
        },
    )
    .await;
    let restarted = app.restart("unroutable-replayed", &[]).await;
    let replay = restarted
        .api()
        .post("/v1/payments/submissions", Auth::Bearer(admin), &request)
        .await;
    assert_status(&replay, StatusCode::ACCEPTED);
    let outcome = replay.parse::<PaymentSubmission>();
    assert_eq!(outcome.payment.id, receipt.payment.id);
    assert_eq!(outcome.state, expected);
    let balance = restarted.wallet_balance(admin, wallet.id).await;
    if ambiguous {
        assert_eq!(balance.available_msat, before.available_msat - 8_000_001_000);
        assert_eq!(balance.reserved_msat, 8_000_001_000);
    } else {
        assert!(outcome.payment.error.is_some());
        assert_eq!(balance.available_msat, before.available_msat);
        assert_eq!(balance.reserved_msat, 0);
    }
}

#[tokio::test]
async fn replaying_an_lnurl_submission_preserves_its_receipt_and_skips_the_callback() {
    let app = TestApp::isolated("command-lnurl", &[]).await;
    let admin = app.admin_token().await;
    let wallet = app.create_wallet(admin, "lnurl-commands").await;
    app.fund_onchain(admin, wallet.id, 100_000).await;
    let mut preimage = [0u8; 32];
    preimage[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    preimage[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    let invoice =
        Counterparty::for_provider(&app.provider).invoice_with_preimage(35_000, &unique("command-lnurl"), &preimage);
    let user = unique("command-payee");
    let mock = MockLnurl::start().await;
    mock.mount_pay_request(&user, 1_000, 1_000_000, 255).await;
    mock.mount_callback_invoice_with_aes(&invoice, "Receipt", "Order paid", &preimage)
        .await;
    let request = json!({"idempotency_key": Uuid::new_v4(), "wallet_id": wallet.id, "input": mock.lnurlp_url(&user), "amount_msat": 35_000, "comment": "order"});
    let r = app
        .api()
        .post("/v1/payments/submissions", Auth::Bearer(admin), &request)
        .await;
    assert_status(&r, StatusCode::ACCEPTED);
    let receipt = r.parse::<PaymentSubmission>();
    wait_until(
        Duration::from_secs(45),
        "LNURL settles with its decrypted receipt",
        || async {
            let current = app
                .api()
                .get(&receipt.status_url, Auth::Bearer(admin))
                .await
                .parse::<PaymentSubmission>();
            current.state == PaymentExecutionState::Settled
                && current
                    .payment
                    .lightning
                    .and_then(|ln| ln.success_action)
                    .and_then(|action| action.message)
                    .as_deref()
                    == Some("Order paid")
        },
    )
    .await;
    let restarted = app.restart("command-lnurl-replay", &[]).await;
    let replay = restarted
        .api()
        .post("/v1/payments/submissions", Auth::Bearer(admin), &request)
        .await;
    assert_status(&replay, StatusCode::ACCEPTED);
    let stored = replay.parse::<PaymentSubmission>();
    assert_eq!(stored.payment.id, receipt.payment.id);
    assert_eq!(stored.state, PaymentExecutionState::Settled);
    assert_eq!(
        stored
            .payment
            .lightning
            .unwrap()
            .success_action
            .unwrap()
            .message
            .as_deref(),
        Some("Order paid")
    );
    assert_eq!(mock.callback_requests().await.len(), 1);
}
