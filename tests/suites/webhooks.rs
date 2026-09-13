//! Account-owned webhook lifecycle, account boundaries, and durable fan-out.

use std::time::Duration;

use reqwest::StatusCode;
use serde_json::json;
use swissknife_types::{CreatedWebhookSubscription, Invoice, WebhookDelivery, WebhookDeliveryStatus};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::TestAccount;
use crate::common::wait::wait_until;
use crate::common::{app, assert_error, assert_status, Auth, TestApp};

async fn ordinary_account(app: &TestApp, label: &str) -> TestAccount {
    let admin = app.admin_token().await;
    let mut account = app.create_account_with_wallet(admin, label).await;
    account.key = app.account_api_key(admin, account.account.id, vec![]).await;
    account
}

#[tokio::test]
async fn ordinary_accounts_can_create_subscriptions_but_anonymous_requests_cannot() {
    let app = app().await;
    let account = ordinary_account(app, "webhook-no-permissions").await;
    let path = format!("/v1/me/wallets/{}/webhooks", account.wallet.id);
    let request = json!({"url": "https://example.com/webhook", "event_types": ["invoice.paid"]});
    assert_error(
        &app.api().post(&path, Auth::None, &request).await,
        StatusCode::UNAUTHORIZED,
    );
    assert_status(
        &app.api().post(&path, Auth::ApiKey(&account.key), &request).await,
        StatusCode::CREATED,
    );
}

#[tokio::test]
async fn keeps_subscriptions_and_secrets_with_the_owning_account() {
    let app = app().await;
    let account = ordinary_account(app, "webhook-owner").await;
    let other = ordinary_account(app, "webhook-other").await;
    let path = format!("/v1/me/wallets/{}/webhooks", account.wallet.id);
    let auth = Auth::ApiKey(&account.key);
    for (request, status) in [
        (
            json!({"url": "http://example.com", "event_types": ["invoice.paid"]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({"url": "https://example.com", "event_types": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({"url": "https://example.com", "event_types": ["unknown"]}),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_error(&app.api().post(&path, auth, request).await, status);
    }
    let request = json!({"url": "https://example.com/webhook", "event_types": ["invoice.paid"]});
    let response = app.api().post(&path, auth, &request).await;
    assert_status(&response, StatusCode::CREATED);
    let created = response.parse::<CreatedWebhookSubscription>();
    let item = format!("{path}/{}", created.subscription.id);
    assert_error(&app.api().post(&path, auth, &request).await, StatusCode::CONFLICT);
    assert_error(
        &app.api().get(&path, Auth::ApiKey(&other.key)).await,
        StatusCode::NOT_FOUND,
    );
    assert_error(
        &app.api().post(&path, Auth::ApiKey(&other.key), request).await,
        StatusCode::NOT_FOUND,
    );
    assert_error(
        &app.api()
            .put(&item, Auth::ApiKey(&other.key), json!({"active": false}))
            .await,
        StatusCode::NOT_FOUND,
    );
    assert_error(
        &app.api()
            .post(&format!("{item}/rotate-secret"), Auth::ApiKey(&other.key), json!({}))
            .await,
        StatusCode::NOT_FOUND,
    );
    assert_error(
        &app.api()
            .get(&format!("{item}/deliveries"), Auth::ApiKey(&other.key))
            .await,
        StatusCode::NOT_FOUND,
    );
    assert_error(
        &app.api().delete(&item, Auth::ApiKey(&other.key)).await,
        StatusCode::NOT_FOUND,
    );

    let subscriptions = app.api().get(&path, auth).await;
    assert_eq!(subscriptions.body.as_array().unwrap().len(), 1);
    assert!(subscriptions.body[0].get("signing_secret").is_none());
    let updated = app.api().put(&item, auth, json!({"active": false})).await;
    assert_status(&updated, StatusCode::OK);
    assert_eq!(updated.body["active"], false);
    assert!(updated.body.get("signing_secret").is_none());
    let rotated = app.api().post(&format!("{item}/rotate-secret"), auth, json!({})).await;
    assert_status(&rotated, StatusCode::OK);
    assert_ne!(rotated.body["signing_secret"], created.signing_secret);
    assert_status(&app.api().delete(&item, auth).await, StatusCode::NO_CONTENT);
    assert_eq!(app.api().get(&path, auth).await.body, json!([]));
}

#[tokio::test]
async fn concurrent_duplicate_urls_return_conflict() {
    let app = app().await;
    let account = ordinary_account(app, "webhook-concurrent-url").await;
    let auth = Auth::ApiKey(&account.key);
    let path = format!("/v1/me/wallets/{}/webhooks", account.wallet.id);
    let api = app.api();
    let request = json!({
        "url": "https://example.com/concurrent-create",
        "event_types": ["invoice.paid"]
    });
    let (first, second) = tokio::join!(api.post(&path, auth, request.clone()), api.post(&path, auth, request));
    let mut statuses = [first.status, second.status];
    statuses.sort_by_key(|status| status.as_u16());
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);

    let first = api
        .post(
            &path,
            auth,
            json!({"url": "https://example.com/update-a", "event_types": ["invoice.paid"]}),
        )
        .await
        .parse::<CreatedWebhookSubscription>();
    let second = api
        .post(
            &path,
            auth,
            json!({"url": "https://example.com/update-b", "event_types": ["invoice.paid"]}),
        )
        .await
        .parse::<CreatedWebhookSubscription>();
    let first_path = format!("{path}/{}", first.subscription.id);
    let second_path = format!("{path}/{}", second.subscription.id);
    let request = json!({"url": "https://example.com/concurrent-update"});
    let (first, second) = tokio::join!(
        api.put(&first_path, auth, request.clone()),
        api.put(&second_path, auth, request)
    );
    let mut statuses = [first.status, second.status];
    statuses.sort_by_key(|status| status.as_u16());
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
}

#[tokio::test]
async fn fans_out_a_real_settlement_and_blocks_private_delivery_destinations() {
    let app = app().await;
    let account = ordinary_account(app, "webhook-settlement").await;
    let auth = Auth::ApiKey(&account.key);
    let path = format!("/v1/me/wallets/{}/webhooks", account.wallet.id);
    let old_invoice = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/invoices", account.wallet.id),
            auth,
            json!({"amount_msat": 1_000_000}),
        )
        .await
        .parse::<Invoice>();
    Counterparty::for_provider(&app.provider).pay(&old_invoice.ln_invoice.as_ref().unwrap().bolt11);
    wait_until(
        Duration::from_secs(60),
        "historical invoice settles before subscription",
        || async {
            app.api()
                .get(
                    &format!("/v1/me/wallets/{}/invoices/{}", account.wallet.id, old_invoice.id),
                    auth,
                )
                .await
                .parse::<Invoice>()
                .status
                == swissknife_types::InvoiceStatus::Settled
        },
    )
    .await;
    let response = app
        .api()
        .post(
            &path,
            auth,
            json!({
                "url": "https://127.0.0.1/private?token=do-not-log-this",
                "event_types": ["invoice.paid"]
            }),
        )
        .await;
    assert_status(&response, StatusCode::CREATED);
    let created = response.parse::<CreatedWebhookSubscription>();
    let invoice = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/invoices", account.wallet.id),
            auth,
            json!({"amount_msat": 1_000_000}),
        )
        .await;
    assert_status(&invoice, StatusCode::OK);
    let invoice = invoice.parse::<Invoice>();
    Counterparty::for_provider(&app.provider).pay(&invoice.ln_invoice.as_ref().unwrap().bolt11);
    let history = format!("{path}/{}/deliveries", created.subscription.id);
    wait_until(
        Duration::from_secs(60),
        "webhook rejects a private destination",
        || async {
            let response = app.api().get(&history, auth).await;
            assert_status(&response, StatusCode::OK);
            let deliveries = response.parse::<Vec<WebhookDelivery>>();
            deliveries
                .first()
                .is_some_and(|delivery| delivery.status == WebhookDeliveryStatus::Exhausted)
        },
    )
    .await;
    let response = app.api().get(&history, auth).await;
    let deliveries = response.parse::<Vec<WebhookDelivery>>();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].attempt_count, 1);
    assert!(deliveries[0].response_status.is_none());
    assert!(!response.body.to_string().contains("do-not-log-this"));
}
