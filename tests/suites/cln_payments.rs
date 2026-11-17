//! CLN reservations follow the submitted payment status, including when no new notification arrives.
use std::time::Duration;

use chrono::{TimeDelta, Utc};

use reqwest::StatusCode;
use sea_orm::{
    sea_query::{Alias, Expr, ExprTrait, Query},
    ConnectionTrait, Database, TransactionTrait,
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

#[tokio::test]
async fn reconciles_a_late_failure_without_releasing_a_different_attempt() {
    if !matrix_cell().1.starts_with("cln_") {
        return;
    }

    let failed_id = Uuid::new_v4();
    let pending_id = Uuid::new_v4();

    // Finish the CLN calls before starting SwissKnife so the test requires a
    // status lookup and does not depend on the delivery or timing of notifications.
    let cln = Counterparty::for_provider("lnd_grpc");
    let mut hashes = Vec::new();
    for (wallet_payment, node_label) in [(failed_id, failed_id), (pending_id, Uuid::new_v4())] {
        let held = Counterparty::for_provider("cln_grpc").hold_invoice(AMOUNT);
        let (bolt11, hash) = (held.bolt11.clone(), held.hash.clone());
        drop(held); // Cancel the invoice, so the real CLN attempt will be rejected.
        cln.pay_cancelled_invoice(&bolt11, &hash, node_label);
        hashes.push((wallet_payment, hash));
    }

    let app = TestApp::isolated("cln-late-failure", &[]).await;
    let token = app.admin_token().await;
    let wallet = app.create_wallet(token, "cln-late-failure").await;
    app.fund_onchain(token, wallet.id, 2 * RESERVED / 1000).await;
    let key = app.account_api_key(token, wallet.account_id, vec![]).await;
    let mut events = app.api().event_stream("/v1/me/events", Auth::ApiKey(&key), None).await;

    // Recreate the durable state after a missed notification. The wallet and
    // funding are provisioned normally and the outcome is observed via the API.
    let database = Database::connect(&app.database_url).await.unwrap();
    let transaction = database.begin().await.unwrap();
    let backend = database.get_database_backend();
    let reserve = Query::update()
        .table(Alias::new("wallet"))
        .value(Alias::new("available_amount"), 0_i64)
        .value(Alias::new("reserved_amount"), (2 * RESERVED) as i64)
        .and_where(Expr::col(Alias::new("id")).eq(wallet.id))
        .to_owned();
    assert_eq!(
        transaction
            .execute_raw(backend.build(&reserve))
            .await
            .unwrap()
            .rows_affected(),
        1
    );
    let mut insert = Query::insert();
    insert.into_table(Alias::new("payment")).columns([
        Alias::new("id"),
        Alias::new("wallet_id"),
        Alias::new("status"),
        Alias::new("ledger"),
        Alias::new("payment_hash"),
        Alias::new("amount_msat"),
        Alias::new("reserved_amount"),
        Alias::new("created_at"),
    ]);
    let now = Utc::now().naive_utc();
    // The newer pending row is checked first. The matching failure is only
    // visible after the sweep has already handled the mismatched record.
    for (id, hash) in hashes {
        let created_at = if id == failed_id {
            now - TimeDelta::seconds(1)
        } else {
            now
        };
        insert.values_panic([
            id.into(),
            wallet.id.into(),
            PaymentStatus::Pending.to_string().into(),
            Ledger::Lightning.to_string().into(),
            hash.into(),
            (AMOUNT as i64).into(),
            (RESERVED as i64).into(),
            created_at.into(),
        ]);
    }
    transaction.execute_raw(backend.build(&insert)).await.unwrap();
    transaction.commit().await.unwrap();

    wait_until(Duration::from_secs(25), "matching CLN failure reconciled", || async {
        let result = app
            .api()
            .get(&format!("/v1/payments/{failed_id}"), Auth::Bearer(token))
            .await;
        assert_status(&result, StatusCode::OK);
        result.parse::<Payment>().status == PaymentStatus::Failed
    })
    .await;

    let pending = app
        .api()
        .get(&format!("/v1/payments/{pending_id}"), Auth::Bearer(token))
        .await;
    assert_status(&pending, StatusCode::OK);
    assert_eq!(pending.parse::<Payment>().status, PaymentStatus::Pending);
    let balance = app.wallet_balance(token, wallet.id).await;
    assert_eq!(balance.available_msat, RESERVED as i64);
    assert_eq!(balance.reserved_msat, RESERVED);

    let event = next_event(&mut events).await;
    assert_eq!(event.payload.event_type, ClientEventType::PaymentFailed);
    assert_eq!(event.payload.resource_id, failed_id);
}
