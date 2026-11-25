use crate::common::{
    assert_error, assert_status, chain::new_address, counterparty::Counterparty, fixtures::unique, wait::wait_until,
    Auth, TestApp,
};
use reqwest::StatusCode;
use serde_json::json;
use std::time::Duration;
use swissknife_types::{IssuedAgentKey, PaymentExecutionState, PaymentSubmission, SpendingPolicy};
use uuid::Uuid;

#[tokio::test]
async fn delegated_spending_obeys_the_account_budget_on_all_rails_and_survives_restart() {
    let app = TestApp::isolated("account-budget", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "budget-owner").await;
    let recipient = app.create_account_with_wallet(admin, "budget-recipient").await;
    let api = app.api();
    let operator = Auth::Bearer(admin);
    let resource = format!("/v1/accounts/{}/spending-policy", owner.account.id);
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let key = api.post(&format!("{wallet}/agent-keys"), Auth::ApiKey(&owner.key),
        json!({"name":"budgeted bot","capabilities":{"spend":true,"read":true},"expires_at": chrono::Utc::now()+chrono::Duration::hours(1)}))
        .await.parse::<IssuedAgentKey>();
    let agent = Auth::WalletKey(&key.key);
    let restricted = app.account_api_key(admin, owner.account.id, vec![]).await;
    let cap = json!({"daily_msat":"50000","per_payment_msat":"49000"});
    assert_error(&api.get(&resource, operator).await, StatusCode::NOT_FOUND);
    assert_error(
        &api.put(&resource, Auth::ApiKey(&restricted), &cap).await,
        StatusCode::FORBIDDEN,
    );
    assert_error(&api.put(&resource, agent, &cap).await, StatusCode::UNAUTHORIZED);
    assert_error(
        &api.put(&resource, operator, json!({"monthly_msat":"18446744073709551615"}))
            .await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_status(&api.put(&resource, operator, &cap).await, StatusCode::NO_CONTENT);
    app.fund_onchain(admin, owner.wallet.id, 50_000).await;
    assert_eq!(
        api.get(&resource, operator)
            .await
            .parse::<SpendingPolicy>()
            .usage
            .daily_msat,
        0
    );

    let submissions = format!("{wallet}/payments/submissions");
    let external = Counterparty::for_provider(&app.provider);
    let invoice = external.invoice(46_000, &unique("capped-invoice"));
    let mut request = json!({"input":invoice,"idempotency_key":Uuid::new_v4(),"maximum_fee_msat":5000});
    let denied = api.post(&submissions, agent, &request).await;
    assert_error(&denied, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(denied.body.to_string().contains("per-payment spending limit"));
    assert_eq!(app.wallet_balance(admin, owner.wallet.id).await.reserved_msat, 0);
    request["maximum_fee_msat"] = json!(1000);
    let accepted = api.post(&submissions, agent, &request).await;
    assert_status(&accepted, StatusCode::ACCEPTED);
    let payment = accepted.parse::<PaymentSubmission>();
    wait_until(Duration::from_secs(40), "budgeted payment settles", || async {
        api.get(&payment.status_url, agent)
            .await
            .parse::<PaymentSubmission>()
            .state
            == PaymentExecutionState::Settled
    })
    .await;
    let settled = api
        .get(&payment.status_url, agent)
        .await
        .parse::<PaymentSubmission>()
        .payment;
    let charged = settled.amount_msat + settled.fee_msat.unwrap_or_default();
    assert_eq!(
        api.post(&submissions, agent, &request)
            .await
            .parse::<PaymentSubmission>()
            .payment
            .id,
        settled.id
    );
    let policy = api.get(&resource, operator).await.parse::<SpendingPolicy>();
    assert_eq!(
        (
            policy.usage.daily_msat,
            policy.usage.weekly_msat,
            policy.usage.monthly_msat
        ),
        (charged, charged, charged)
    );

    let internal_address = api
        .post(
            "/v1/bitcoin/addresses",
            operator,
            json!({"wallet_id":recipient.wallet.id}),
        )
        .await;
    let internal_address = internal_address.body["address"].as_str().unwrap();
    let internal =
        json!({"idempotency_key":Uuid::new_v4(),"input":internal_address,"amount_msat":5000,"maximum_fee_msat":0});
    let denied = api.post(&submissions, agent, &internal).await;
    assert_error(&denied, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        denied.body.to_string().contains("daily spending limit"),
        "{}",
        denied.body
    );
    let external_address = new_address().await;
    let withdrawal = json!({"idempotency_key":Uuid::new_v4(),"input":external_address,"amount_msat":2_000_000,"maximum_fee_msat":500_000});
    let denied = api.post(&submissions, agent, withdrawal).await;
    assert_error(&denied, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(denied.body.to_string().contains("spending limit"), "{}", denied.body);

    let restart = app.restart("budget-restart", &[]).await;
    let api = restart.api();
    assert_eq!(
        api.get(&resource, operator)
            .await
            .parse::<SpendingPolicy>()
            .usage
            .daily_msat,
        charged
    );
    assert_status(&api.delete(&resource, operator).await, StatusCode::NO_CONTENT);
    assert_error(&api.get(&resource, operator).await, StatusCode::NOT_FOUND);
    assert_status(
        &api.put(&resource, operator, json!({"monthly_msat":"9007199254740993"}))
            .await,
        StatusCode::NO_CONTENT,
    );
    let reread = api.get(&resource, operator).await;
    assert_eq!(reread.body["limits"]["monthly_msat"], "9007199254740993");
    assert_eq!(reread.parse::<SpendingPolicy>().usage.monthly_msat, charged);
}
