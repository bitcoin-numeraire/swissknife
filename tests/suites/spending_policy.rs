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
        (u128::from(charged), u128::from(charged), u128::from(charged))
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
        u128::from(charged)
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
    assert_eq!(reread.parse::<SpendingPolicy>().usage.monthly_msat, u128::from(charged));
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

#[tokio::test]
async fn rolling_velocity_is_atomic_per_credential_and_does_not_limit_owner_payments() {
    let app = TestApp::isolated("agent-velocity", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "agent-rate").await;
    let recipient = app.create_account_with_wallet(admin, "recipient-rate").await;
    let api = app.api();
    let owner_auth = Auth::ApiKey(&owner.key);
    let policy = format!("/v1/accounts/{}/spending-policy", owner.account.id);
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let path = format!("{wallet}/payments/submissions");
    let mut credentials = Vec::new();
    for name in ["worker-a", "worker-b"] {
        let result = api
            .post(
                &format!("{wallet}/agent-keys"),
                owner_auth,
                json!({
                    "name":name,"capabilities":{"spend":true,"read":true},
                    "expires_at":chrono::Utc::now()+chrono::Duration::hours(2)
                }),
            )
            .await;
        assert_status(&result, StatusCode::CREATED);
        credentials.push(result.parse::<IssuedAgentKey>());
    }
    let operator = Auth::Bearer(admin);
    let cap = json!({"agent_hourly_msat":"10000", "agent_hourly_warning_msat":"10000"});
    assert_status(&api.put(&policy, operator, &cap).await, StatusCode::NO_CONTENT);
    assert_eq!(
        api.get(&policy, operator).await.body["limits"]["agent_hourly_msat"],
        "10000"
    );
    app.fund_onchain(admin, owner.wallet.id, 10_000).await;
    let address = api
        .post(
            "/v1/bitcoin/addresses",
            operator,
            json!({"wallet_id":recipient.wallet.id}),
        )
        .await
        .body["address"]
        .as_str()
        .unwrap()
        .to_owned();
    let request = |amount: u64| json!({"input":address,"idempotency_key":Uuid::new_v4(),"amount_msat":amount,"maximum_fee_msat":0});
    let first = Auth::WalletKey(&credentials[0].key);
    let second = Auth::WalletKey(&credentials[1].key);
    let requests: Vec<_> = (0..6).map(|_| request(6000)).collect();
    let results = futures_util::future::join_all(requests.iter().map(|r| api.post(&path, first, r))).await;
    let mut admitted = Vec::new();
    for (i, response) in results.iter().enumerate() {
        if response.status == StatusCode::ACCEPTED {
            admitted.push(i);
        } else {
            assert_error(response, StatusCode::UNPROCESSABLE_ENTITY);
            assert!(response.body.to_string().contains("rolling hourly"));
        }
    }
    assert_eq!(admitted.len(), 1);
    assert_status(&api.post(&path, first, request(4000)).await, StatusCode::ACCEPTED);
    assert_error(
        &api.post(&path, first, request(1000)).await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_status(&api.post(&path, second, request(10000)).await, StatusCode::ACCEPTED);
    assert_status(&api.post(&path, owner_auth, request(11000)).await, StatusCode::ACCEPTED);
    let app = app.restart("velocity-restart", &[]).await;
    let api = app.api();
    assert_error(
        &api.post(&path, first, request(1000)).await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    let replay = api.post(&path, first, &requests[admitted[0]]).await;
    assert_status(&replay, StatusCode::ACCEPTED);
    assert_eq!(replay.body["payment"]["id"], results[admitted[0]].body["payment"]["id"]);
    assert_eq!(
        app.wallet_balance(app.admin_token().await, recipient.wallet.id)
            .await
            .available_msat,
        31000
    );
    let activity = format!("/v1/accounts/{}/agent-spending-activity", owner.account.id);
    let operator = Auth::Bearer(app.admin_token().await);
    let restricted = app
        .account_api_key(app.admin_token().await, owner.account.id, vec![])
        .await;
    assert_error(
        &api.get(&activity, Auth::ApiKey(&restricted)).await,
        StatusCode::FORBIDDEN,
    );
    assert_error(&api.get(&activity, first).await, StatusCode::UNAUTHORIZED);
    assert_error(
        &api.get(&format!("{activity}?limit=101"), operator).await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    let page = api
        .get(&format!("{activity}?limit=1"), operator)
        .await
        .parse::<swissknife_types::AgentActivityPage>();
    assert_eq!(page.items.len(), 1);
    let after = page.next_after.unwrap();
    assert_eq!(after, page.items[0].agent_key_id);
    let next = api
        .get(&format!("{activity}?limit=1&after={after}"), operator)
        .await
        .parse::<swissknife_types::AgentActivityPage>();
    assert!(next.next_after.is_none());
    assert_eq!(next.items.len(), 1);
    assert!(next.items[0].agent_key_id > after);
    for row in page.items.iter().chain(&next.items) {
        let credential = credentials.iter().find(|k| k.metadata.id == row.agent_key_id).unwrap();
        assert_eq!(row.name.as_deref(), Some(credential.metadata.name.as_str()));
        assert_eq!(row.wallet_id, Some(owner.wallet.id));
        assert_eq!(row.current_hour.charged_msat, 10000);
        assert!(!row.above_hourly_warning, "equality must not warn");
        assert_eq!(
            row.current_hour.payments,
            if row.agent_key_id == credentials[0].metadata.id {
                2
            } else {
                1
            }
        );
        assert_eq!((row.previous_hour.payments, row.previous_hour.charged_msat), (0, 0));
    }
    // A warning can be the only policy rule. Crossing it admits the payment
    // and flags only the credential that spent the extra amount.
    let warning = |amount: &str| json!({"agent_hourly_warning_msat": amount});
    assert_error(
        &api.put(&policy, Auth::ApiKey(&restricted), &warning("0")).await,
        StatusCode::FORBIDDEN,
    );
    assert_error(
        &api.put(&policy, operator, &warning("9223372036854775808")).await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_status(
        &api.put(&policy, operator, &warning("10000")).await,
        StatusCode::NO_CONTENT,
    );
    assert_status(&api.post(&path, second, request(1000)).await, StatusCode::ACCEPTED);
    let warnings = api
        .get(&activity, operator)
        .await
        .parse::<swissknife_types::AgentActivityPage>();
    assert_eq!(warnings.items.len(), 2);
    for row in warnings.items {
        assert_eq!(row.above_hourly_warning, row.agent_key_id == credentials[1].metadata.id);
    }
    assert_status(&api.put(&policy, operator, &warning("0")).await, StatusCode::NO_CONTENT);
    let revoke = format!("{wallet}/agent-keys/{}", credentials[0].metadata.id);
    assert_status(&api.delete(&revoke, owner_auth).await, StatusCode::NO_CONTENT);
    let all = api
        .get(&activity, operator)
        .await
        .parse::<swissknife_types::AgentActivityPage>();
    let revoked = all
        .items
        .iter()
        .find(|row| row.agent_key_id == credentials[0].metadata.id)
        .unwrap();
    assert_eq!(revoked.name, None);
    assert_eq!(revoked.current_hour.charged_msat, 10000);
    assert!(revoked.above_hourly_warning);
    assert_status(&api.delete(&policy, operator).await, StatusCode::NO_CONTENT);
    let disabled = api
        .get(&activity, operator)
        .await
        .parse::<swissknife_types::AgentActivityPage>();
    assert_eq!(disabled.items.len(), 2);
    assert!(disabled.items.iter().all(|row| !row.above_hourly_warning));
    assert_status(
        &api.put(&policy, operator, &json!({"daily_msat":"55000"})).await,
        StatusCode::NO_CONTENT,
    );
    assert!(api.get(&policy, operator).await.body["limits"]
        .get("agent_hourly_warning_msat")
        .is_none());
    assert!(api
        .get(&activity, operator)
        .await
        .parse::<swissknife_types::AgentActivityPage>()
        .items
        .iter()
        .all(|row| !row.above_hourly_warning));
    let other = format!("/v1/accounts/{}/agent-spending-activity", recipient.account.id);
    assert!(api
        .get(&other, operator)
        .await
        .parse::<swissknife_types::AgentActivityPage>()
        .items
        .is_empty());
    app.stop();
}
