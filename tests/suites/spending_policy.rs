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
    let report = format!("/v1/accounts/{}/spending-report", owner.account.id);
    let rows = api.get(&report, operator).await;
    assert_status(&rows, StatusCode::OK);
    let first = &rows.body[0];
    assert_eq!(first["payments"], 1);
    assert_eq!(first["charged_msat"], charged.to_string());
    let date = first["period_start"].as_str().unwrap();
    let filtered = format!(
        "{report}?from={date}&through={date}&wallet_id={}&agent_key_id={}",
        owner.wallet.id, key.metadata.id
    );
    assert_eq!(api.get(&filtered, operator).await.body, rows.body);
    let monday = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .unwrap()
        .week(chrono::Weekday::Mon)
        .first_day();
    assert_eq!(
        api.get(&format!("{filtered}&interval=week"), operator).await.body[0]["period_start"],
        monday.to_string()
    );
    for filter in [
        format!("wallet_id={}", recipient.wallet.id),
        format!("agent_key_id={}", Uuid::new_v4()),
    ] {
        assert_eq!(api.get(&format!("{report}?{filter}"), operator).await.body, json!([]));
    }
    assert_error(
        &api.get(&format!("{report}?from=2020-01-01&through=2025-01-01"), operator)
            .await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_error(&api.get(&report, agent).await, StatusCode::UNAUTHORIZED);
    assert_error(
        &api.get(&report, Auth::ApiKey(&restricted)).await,
        StatusCode::FORBIDDEN,
    );
    let revoke = format!("{wallet}/agent-keys/{}", key.metadata.id);
    assert_status(
        &api.delete(&revoke, Auth::ApiKey(&owner.key)).await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(api.get(&filtered, operator).await.body, rows.body);
    let csv = reqwest::Client::new()
        .get(format!("{}{report}.csv", restart.base_url))
        .bearer_auth(admin)
        .send()
        .await
        .unwrap();
    assert_eq!(csv.status(), StatusCode::OK);
    assert_eq!(csv.headers()["content-type"], "text/csv; charset=utf-8");
    assert_eq!(
        csv.text().await.unwrap(),
        format!("period_start,payments,charged_msat\r\n{date},1,{charged}\r\n")
    );
}

#[tokio::test]
async fn delegated_hours_apply_without_a_budget_and_owner_payments_stay_independent() {
    use chrono::{Datelike, Duration as ChronoDuration, Utc};
    let app = TestApp::isolated("agent-hours", &[]).await;
    let admin = app.admin_token().await;
    let operator = Auth::Bearer(admin);
    let owner = app.create_account_with_wallet(admin, "scheduled-agent").await;
    let api = app.api();
    let owner_auth = Auth::ApiKey(&owner.key);
    let path = format!("/v1/accounts/{}/agent-spending-hours", owner.account.id);
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let payments = format!("{wallet}/payments/submissions");
    let key = api
        .post(
            &format!("{wallet}/agent-keys"),
            owner_auth,
            json!({
                "name":"scheduled agent","capabilities":{"spend":true,"read":true},
                "expires_at":Utc::now()+ChronoDuration::hours(1)
            }),
        )
        .await
        .parse::<IssuedAgentKey>();
    let agent = Auth::WalletKey(&key.key);
    let now = Utc::now();
    let closed = json!({"weekdays":[(now+ChronoDuration::days(3)).weekday().number_from_monday()],"start_minute":0,"end_minute":1440});
    let open = json!({"weekdays":[1,2,3,4,5,6,7],"start_minute":0,"end_minute":1440});
    assert_error(&api.get(&path, operator).await, StatusCode::NOT_FOUND);
    assert_error(&api.put(&path, agent, &open).await, StatusCode::UNAUTHORIZED);
    let plain_key = app.account_api_key(admin, owner.account.id, vec![]).await;
    assert_error(
        &api.put(&path, Auth::ApiKey(&plain_key), &open).await,
        StatusCode::FORBIDDEN,
    );
    for invalid in [
        json!({"weekdays":[1,1],"start_minute":0,"end_minute":1440}),
        json!({"weekdays":[8],"start_minute":0,"end_minute":1440}),
        json!({"weekdays":[2],"start_minute":1440,"end_minute":10}),
        json!({"weekdays":[3],"start_minute":5,"end_minute":5}),
    ] {
        assert_error(
            &api.put(&path, operator, invalid).await,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }
    assert_status(&api.put(&path, operator, &closed).await, StatusCode::NO_CONTENT);
    app.fund_onchain(admin, owner.wallet.id, 50_000).await;
    let app = app.restart("scheduled", &[]).await;
    let api = app.api();
    assert_eq!(api.get(&path, operator).await.body, closed);
    let external = Counterparty::for_provider(&app.provider);
    let make_request = |label: &str| json!({"input":external.invoice(17_000,&unique(label)),"idempotency_key":Uuid::new_v4(),"maximum_fee_msat":1000});
    let request = make_request("scheduled-paid");
    let denied = api.post(&payments, agent, &request).await;
    assert_error(&denied, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(denied.body.to_string().contains("UTC spending hours"));
    assert_eq!(
        app.wallet_balance(app.admin_token().await, owner.wallet.id)
            .await
            .reserved_msat,
        0
    );
    let own = api.post(&payments, owner_auth, make_request("owner-paid")).await;
    assert_status(&own, StatusCode::ACCEPTED);
    assert_status(&api.put(&path, operator, &open).await, StatusCode::NO_CONTENT);
    let admitted = api.post(&payments, agent, &request).await;
    assert_status(&admitted, StatusCode::ACCEPTED);
    assert_status(&api.put(&path, operator, &closed).await, StatusCode::NO_CONTENT);
    for response in [admitted, own] {
        let submission = response.parse::<PaymentSubmission>();
        wait_until(
            Duration::from_secs(40),
            "admitted payment settles outside hours",
            || async {
                api.get(&submission.status_url, owner_auth)
                    .await
                    .parse::<PaymentSubmission>()
                    .state
                    == PaymentExecutionState::Settled
            },
        )
        .await;
    }
    let replay = api.post(&payments, agent, &request).await;
    assert_eq!(
        replay.parse::<PaymentSubmission>().state,
        PaymentExecutionState::Settled
    );
    assert_status(&api.delete(&path, operator).await, StatusCode::NO_CONTENT);
    assert_error(&api.get(&path, operator).await, StatusCode::NOT_FOUND);
    app.stop();
}
