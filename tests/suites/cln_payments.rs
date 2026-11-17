//! Real CLN attempt notifications must not release an in-flight payment's funds.
use std::time::Duration;

use reqwest::StatusCode;
use sea_orm::{
    sea_query::{Alias, Query},
    ConnectionTrait, Database,
};
use swissknife_types::{ClientEventType, Ledger, Payment, PaymentStatus, SendPaymentRequest};
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

#[tokio::test]
async fn failed_part_cannot_release_funds_for_a_second_spend() {
    if !matrix_cell().1.starts_with("cln_") {
        return;
    }
    let app = TestApp::isolated("cln-failed-part", &[]).await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "cln-failed-part").await;
    app.fund_onchain(token, wallet.id, RESERVED / 1000).await;
    let key = app.account_api_key(token, wallet.account_id, vec![]).await;
    let wallet = wallet.id;
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
    assert_eq!(payment(&app, wallet).await.status, PaymentStatus::Pending);
    let balance = app.wallet_balance(app.admin_token().await, wallet).await;
    assert_eq!(balance.available_msat, 0);
    assert_eq!(balance.reserved_msat, RESERVED);
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

async fn store_legacy_failed_payment(app: &TestApp, id: Uuid, wallet_id: Uuid, payment_hash: &str) {
    // Model a failed payment written before the BOLT11 column was added.
    let database = Database::connect(&app.database_url).await.unwrap();
    let statement = Query::insert()
        .into_table(Alias::new("payment"))
        .columns([
            Alias::new("id"),
            Alias::new("wallet_id"),
            Alias::new("status"),
            Alias::new("ledger"),
            Alias::new("payment_hash"),
            Alias::new("amount_msat"),
        ])
        .values_panic([
            id.into(),
            wallet_id.into(),
            PaymentStatus::Failed.to_string().into(),
            Ledger::Lightning.to_string().into(),
            payment_hash.into(),
            (AMOUNT as i64).into(),
        ])
        .to_owned();
    database
        .execute_raw(database.get_database_backend().build(&statement))
        .await
        .unwrap();
}

#[tokio::test]
async fn recovers_and_retries_a_failed_invoice_from_the_original_wallet() {
    let (_, provider) = matrix_cell();
    if !provider.starts_with("cln_") {
        return;
    }

    let previous_id = Uuid::new_v4();
    let mut held = Counterparty::for_provider(&provider).hold_invoice(AMOUNT);
    Counterparty::for_provider("lnd_grpc").record_rejected_payment(&held.bolt11, previous_id);

    let app = TestApp::isolated("cln-retry", &[]).await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "cln-retry").await;
    let other_wallet = app.create_wallet(token, "cln-wrong-retry-wallet").await;
    app.fund_onchain(token, wallet.id, RESERVED / 1000).await;
    let key = app.account_api_key(token, wallet.account_id, vec![]).await;

    store_legacy_failed_payment(&app, previous_id, wallet.id, &held.hash).await;

    let path = format!("/v1/me/wallets/{}/payments", wallet.id);
    let details = app
        .api()
        .get(&format!("{path}/{previous_id}"), Auth::ApiKey(&key))
        .await;
    assert_status(&details, StatusCode::OK);
    let lightning = details.parse::<Payment>().lightning.unwrap();
    assert_eq!(lightning.bolt11.as_deref(), Some(held.bolt11.as_str()));
    assert_eq!(lightning.retryable, Some(true));

    let wrong_wallet = app
        .api()
        .post(
            "/v1/payments",
            Auth::Bearer(token),
            request(other_wallet.id, &held.bolt11),
        )
        .await;
    assert_error(&wrong_wallet, StatusCode::CONFLICT);

    let api = app.api();
    let send = request(wallet.id, &held.bolt11);
    let retry_path = path.clone();
    let retry_key = key.clone();
    let retry = tokio::spawn(async move { api.post(&retry_path, Auth::ApiKey(&retry_key), send).await });
    wait_until(
        Duration::from_secs(10),
        "CLN retried on the original wallet",
        || async { held.accepted() },
    )
    .await;

    let pending = payment(&app, wallet.id).await;
    assert_eq!(pending.id, previous_id);
    assert_eq!(pending.status, PaymentStatus::Pending);
    assert_eq!(app.wallet_balance(token, wallet.id).await.reserved_msat, RESERVED);
    let duplicate = app
        .api()
        .post(&path, Auth::ApiKey(&key), request(wallet.id, &held.bolt11))
        .await;
    assert_error(&duplicate, StatusCode::CONFLICT);

    held.settle();
    let response = retry.await.unwrap();
    assert_status(&response, StatusCode::OK);
    assert_eq!(response.parse::<Payment>().status, PaymentStatus::Settled);
    let settled = payment(&app, wallet.id).await;
    assert_eq!(settled.id, previous_id);
    assert_eq!(settled.lightning.unwrap().bolt11.as_deref(), Some(held.bolt11.as_str()));
    let balance = app.wallet_balance(token, wallet.id).await;
    assert_eq!(balance.available_msat, (RESERVED - AMOUNT) as i64);
    assert_eq!(balance.reserved_msat, 0);

    let settled_duplicate = app
        .api()
        .post(&path, Auth::ApiKey(&key), request(wallet.id, &held.bolt11))
        .await;
    assert_error(&settled_duplicate, StatusCode::CONFLICT);
}

#[tokio::test]
async fn copies_the_invoice_but_blocks_retry_after_another_cln_attempt_settled() {
    let (_, provider) = matrix_cell();
    if !provider.starts_with("cln_") {
        return;
    }

    let app = TestApp::isolated("cln-retry-duplicate", &[]).await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "cln-retry-duplicate").await;
    app.fund_onchain(token, wallet.id, RESERVED / 1000).await;

    let previous_id = Uuid::new_v4();
    let invoice = Counterparty::for_provider(&provider).invoice(AMOUNT, "retry-duplicate");
    let cln = Counterparty::for_provider("lnd_grpc");
    let hash = cln.record_rejected_payment(&invoice, previous_id);
    cln.pay(&invoice);
    store_legacy_failed_payment(&app, previous_id, wallet.id, &hash).await;

    let details = app
        .api()
        .get(&format!("/v1/payments/{previous_id}"), Auth::Bearer(token))
        .await;
    assert_status(&details, StatusCode::OK);
    let lightning = details.parse::<Payment>().lightning.unwrap();
    assert_eq!(lightning.bolt11.as_deref(), Some(invoice.as_str()));
    assert_ne!(lightning.retryable, Some(true));

    let duplicate = app
        .api()
        .post("/v1/payments", Auth::Bearer(token), request(wallet.id, &invoice))
        .await;
    assert_error(&duplicate, StatusCode::CONFLICT);
    assert_eq!(payment(&app, wallet.id).await.status, PaymentStatus::Failed);
    let balance = app.wallet_balance(token, wallet.id).await;
    assert_eq!(balance.available_msat, RESERVED as i64);
    assert_eq!(balance.reserved_msat, 0);
}
