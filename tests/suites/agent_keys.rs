use crate::common::{
    assert_error, assert_status,
    counterparty::Counterparty,
    fixtures::{mainnet_btc_asset_id, unique},
    wait::wait_until,
    Auth, TestApp,
};
use chrono::{Duration as ChronoDuration, Utc};
use reqwest::StatusCode;
use serde_json::json;
use std::time::Duration;
use swissknife_types::{AgentKey, IssuedAgentKey, PaymentExecutionState, PaymentSubmission, Wallet};
use uuid::Uuid;

#[tokio::test]
async fn owner_delegates_only_selected_operations_on_one_wallet() {
    let app = TestApp::isolated("agent-keys", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "agent-owner").await;
    let other = app.create_account_with_wallet(admin, "agent-foreign").await;
    let account = Auth::ApiKey(&owner.key);
    let api = app.api();
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let keys = format!("{wallet}/agent-keys");
    let sibling = api
        .post("/v1/me/wallets", account, json!({"asset_id":mainnet_btc_asset_id()}))
        .await
        .parse::<Wallet>();
    let template = |caps: serde_json::Value| json!({"name":"office integration","capabilities":caps,"expires_at":Utc::now()+ChronoDuration::days(1)});
    let read_req = template(json!({"read":true}));
    assert_error(
        &api.post(&keys, Auth::ApiKey(&other.key), &read_req).await,
        StatusCode::NOT_FOUND,
    );
    assert_error(
        &api.post(&keys, account, template(json!({}))).await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    let new = api.post(&keys, account, &read_req).await;
    assert_status(&new, StatusCode::CREATED);
    let reader = new.parse::<IssuedAgentKey>();
    let read = Auth::WalletKey(&reader.key);
    let stored = api.get(&keys, account).await;
    assert!(stored.body[0].get("key").is_none());
    assert!(stored.body[0].get("key_hash").is_none());
    assert_eq!(stored.parse::<Vec<AgentKey>>()[0].id, reader.metadata.id);
    assert_status(&api.get(&format!("{wallet}/balance"), read).await, StatusCode::OK);
    for id in [sibling.id, other.wallet.id] {
        assert_error(
            &api.get(&format!("/v1/me/wallets/{id}/balance"), read).await,
            StatusCode::UNAUTHORIZED,
        );
    }
    for route in [&keys, "/v1/me", "/v1/api-keys", "/v1/me/events"] {
        assert_error(&api.get(route, read).await, StatusCode::UNAUTHORIZED);
    }
    assert_error(
        &api.get(&format!("{wallet}/balance"), Auth::ApiKey(&reader.key)).await,
        StatusCode::UNAUTHORIZED,
    );
    let amount = json!({"amount_msat":32000,"description":"delegated receipt"});
    assert_error(
        &api.post(&format!("{wallet}/invoices"), read, &amount).await,
        StatusCode::FORBIDDEN,
    );
    let receive = api
        .post(&keys, account, template(json!({"receive":true})))
        .await
        .parse::<IssuedAgentKey>();
    let invoice = api
        .post(&format!("{wallet}/invoices"), Auth::WalletKey(&receive.key), &amount)
        .await;
    assert_status(&invoice, StatusCode::OK);
    assert_eq!(invoice.body["wallet_id"], owner.wallet.id.to_string());
    assert_error(
        &api.get(&format!("{wallet}/balance"), Auth::WalletKey(&receive.key))
            .await,
        StatusCode::FORBIDDEN,
    );

    let spend = api
        .post(&keys, account, template(json!({"spend":true})))
        .await
        .parse::<IssuedAgentKey>();
    app.fund_onchain(admin, owner.wallet.id, 25_000).await;
    let dest = Counterparty::for_provider(&app.provider).invoice(32000, &unique("agent-key-pay"));
    let submit = json!({"input":dest,"idempotency_key":Uuid::new_v4(),"maximum_fee_msat":2000});
    let pay = format!("{wallet}/payments/submissions");
    assert_error(&api.post(&pay, read, &submit).await, StatusCode::FORBIDDEN);
    let authorized = api.post(&pay, Auth::WalletKey(&spend.key), &submit).await;
    assert_status(&authorized, StatusCode::ACCEPTED);
    let command = authorized.parse::<PaymentSubmission>();
    let repeat = api
        .post(&pay, Auth::WalletKey(&spend.key), &submit)
        .await
        .parse::<PaymentSubmission>();
    assert_eq!(repeat.payment.id, command.payment.id);
    wait_until(Duration::from_secs(30), "delegate payment settles", || async {
        api.get(&command.status_url, read)
            .await
            .parse::<PaymentSubmission>()
            .state
            == PaymentExecutionState::Settled
    })
    .await;
    let resumed = app.restart("delegation-restart", &[]).await;
    let api = resumed.api();
    assert_status(&api.get(&format!("{wallet}/balance"), read).await, StatusCode::OK);
    assert_status(
        &api.delete(&format!("{keys}/{}", reader.metadata.id), account).await,
        StatusCode::NO_CONTENT,
    );
    assert_error(
        &api.get(&format!("{wallet}/balance"), read).await,
        StatusCode::UNAUTHORIZED,
    );
    let mut expiring = read_req;
    expiring["expires_at"] = json!(Utc::now() + ChronoDuration::seconds(2));
    let short = api.post(&keys, account, expiring).await.parse::<IssuedAgentKey>();
    wait_until(Duration::from_secs(6), "wallet credential expires", || async {
        api.get(&format!("{wallet}/balance"), Auth::WalletKey(&short.key))
            .await
            .status
            == StatusCode::UNAUTHORIZED
    })
    .await;
    assert!(!resumed.logs_contain(&short.key));
    assert!(!app.logs_contain(&spend.key));
}

#[tokio::test]
async fn saved_contacts_have_independent_permissions_and_survive_restarts() {
    let app = TestApp::isolated("saved-wallet-contacts", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "contact-owner").await;
    let other = app.create_account_with_wallet(admin, "other-contact-owner").await;
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let keys = format!("{wallet}/agent-keys");
    let saved = format!("{wallet}/saved-contacts");
    let other_saved = format!("/v1/me/wallets/{}/saved-contacts", other.wallet.id);
    let api = app.api();
    let account = Auth::ApiKey(&owner.key);
    let issue =
        |caps| json!({"name":"contact key","expires_at":Utc::now()+ChronoDuration::hours(1),"capabilities":caps});
    let editor: IssuedAgentKey = api
        .post(&keys, account, issue(json!({"manage_contacts":true})))
        .await
        .parse();
    let previous: IssuedAgentKey = api
        .post(&keys, account, issue(json!({"read":true,"receive":true,"spend":true})))
        .await
        .parse();
    assert!(!previous.metadata.capabilities.manage_contacts);
    let edit = Auth::WalletKey(&editor.key);
    let read = Auth::WalletKey(&previous.key);
    let body = json!({"ln_address":"Alice@EXAMPLE.COM","label":" Alice "});
    assert_error(&api.post(&saved, read, &body).await, StatusCode::FORBIDDEN);
    assert_error(&api.get(&saved, edit).await, StatusCode::FORBIDDEN);
    assert_error(&api.post(&other_saved, edit, &body).await, StatusCode::UNAUTHORIZED);
    assert_error(&api.post(&other_saved, account, &body).await, StatusCode::NOT_FOUND);
    assert_error(
        &api.post(&saved, edit, json!({"ln_address":"alice@exam/ple.com"})).await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_error(
        &api.post(
            &saved,
            edit,
            json!({"ln_address":"bob@example.com","label":"x".repeat(101)}),
        )
        .await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    let (one, two) = tokio::join!(api.post(&saved, edit, &body), api.post(&saved, edit, &body));
    assert_status(&one, StatusCode::OK);
    assert_status(&two, StatusCode::OK);
    let first: swissknife_types::SavedContact = one.parse();
    assert_eq!(first, two.parse());
    assert_eq!(first.ln_address, "Alice@example.com");
    assert_eq!(first.label.as_deref(), Some("Alice"));
    let updated: swissknife_types::SavedContact = api
        .post(
            &saved,
            account,
            json!({"ln_address":"Alice@example.com","label":"Billing"}),
        )
        .await
        .parse();
    assert_eq!(updated.saved_at, first.saved_at);
    assert_eq!(updated.label.as_deref(), Some("Billing"));
    let restarted = app.restart("saved-contacts-readback", &[]).await;
    let api = restarted.api();
    let book: Vec<swissknife_types::SavedContact> = api.get(&saved, read).await.parse();
    assert_eq!(book, vec![updated]);
    let history: Vec<swissknife_types::Contact> = api.get(&format!("{wallet}/contacts"), read).await.parse();
    assert!(history.is_empty());
    let target = format!("{saved}/Alice@example.com");
    assert_error(&api.delete(&target, read).await, StatusCode::FORBIDDEN);
    assert_status(&api.delete(&target, edit).await, StatusCode::NO_CONTENT);
    let empty: Vec<swissknife_types::SavedContact> = api.get(&saved, read).await.parse();
    assert!(empty.is_empty());
    assert_status(
        &api.delete(&format!("{keys}/{}", editor.metadata.id), account).await,
        StatusCode::NO_CONTENT,
    );
    assert_error(&api.post(&saved, edit, &body).await, StatusCode::UNAUTHORIZED);
}
