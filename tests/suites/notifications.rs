//! The inbox projects real committed settlements and keeps presentation state account scoped.
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::json;
use swissknife_types::{ClientEventType, Invoice, Ledger, NewInvoiceRequest, NotificationPage};

use crate::common::{
    assert_error, assert_status, counterparty::Counterparty, fixtures::TestAccount, wait::wait_until, Auth, TestApp,
};

const PATH: &str = "/v1/me/notifications";

async fn page(app: &TestApp, key: &str, query: &str) -> NotificationPage {
    let response = app.api().get(&format!("{PATH}{query}"), Auth::ApiKey(key)).await;
    assert_status(&response, StatusCode::OK);
    response.parse()
}

async fn receive(app: &TestApp, account: &TestAccount, msat: u64) -> Invoice {
    let response = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/invoices", account.wallet.id),
            Auth::ApiKey(&account.key),
            NewInvoiceRequest {
                wallet_id: None,
                amount_msat: msat,
                description: Some("Inbox receipt".into()),
                expiry: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::OK);
    let invoice = response.parse::<Invoice>();
    Counterparty::for_provider(&app.provider).pay(&invoice.ln_invoice.as_ref().unwrap().bolt11);
    invoice
}

#[tokio::test]
async fn account_inbox_paginates_marks_and_archives_only_owned_settlements() {
    let app = TestApp::isolated("notification-center", &[]).await;
    let admin = app.admin_token().await;
    let mut alice = app.create_account_with_wallet(admin, "inbox-alice").await;
    alice.key = app.account_api_key(admin, alice.account.id, vec![]).await;
    let bob = app.create_account_with_wallet(admin, "inbox-bob").await;
    let api = app.api();
    assert_error(&api.get(PATH, Auth::None).await, StatusCode::UNAUTHORIZED);
    assert_error(
        &api.get(&format!("{PATH}?limit=0"), Auth::ApiKey(&alice.key)).await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_eq!(page(&app, &alice.key, "").await.unread_count, 0);
    let first = receive(&app, &alice, 21_000).await;
    wait_until(Duration::from_secs(45), "first inbox receipt", || async {
        page(&app, &alice.key, "").await.unread_count == 1
    })
    .await;
    let second = receive(&app, &alice, 32_000).await;
    wait_until(Duration::from_secs(45), "second inbox receipt", || async {
        page(&app, &alice.key, "").await.unread_count == 2
    })
    .await;
    let latest = page(&app, &alice.key, "?limit=1").await;
    assert_eq!(latest.items.len(), 1);
    let new = &latest.items[0];
    assert_eq!(new.event_type, ClientEventType::InvoicePaid);
    assert_eq!(new.wallet_id, alice.wallet.id);
    assert_eq!(new.resource_id, second.id);
    assert_eq!(new.ledger, Some(Ledger::Lightning));
    assert_eq!(new.amount_msat.as_deref(), Some("32000"));
    assert_eq!(new.description.as_deref(), Some("Inbox receipt"));
    assert_eq!(latest.next_before, Some(new.source_event_id));
    let older = page(&app, &alice.key, &format!("?limit=1&before={}", new.source_event_id)).await;
    assert_eq!(older.items[0].resource_id, first.id);
    assert!(older.next_before.is_none());
    let old_id = older.items[0].source_event_id;
    let path = format!("{PATH}/{old_id}");
    assert_eq!(page(&app, &bob.key, "").await.unread_count, 0);
    assert_error(
        &api.patch(&path, Auth::ApiKey(&bob.key), json!({"action":"archive"}))
            .await,
        StatusCode::NOT_FOUND,
    );
    // A mark-all issued from an older page cannot clear a later notification.
    assert_status(
        &api.post(
            &format!("{PATH}/read"),
            Auth::ApiKey(&alice.key),
            json!({"through":old_id}),
        )
        .await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(page(&app, &alice.key, "").await.unread_count, 1);
    for _ in 0..2 {
        assert_status(
            &api.patch(&path, Auth::ApiKey(&alice.key), json!({"action":"archive"}))
                .await,
            StatusCode::NO_CONTENT,
        );
    }
    let new_path = format!("{PATH}/{}", new.source_event_id);
    assert_status(
        &api.patch(&new_path, Auth::ApiKey(&alice.key), json!({"action":"read"}))
            .await,
        StatusCode::NO_CONTENT,
    );
    let restarted = app.restart("notification-center-replayed", &[]).await;
    let active = page(&restarted, &alice.key, "").await;
    assert_eq!(active.unread_count, 0);
    assert_eq!(active.items.len(), 1);
    assert!(active.items[0].is_read);
    let archived = page(&restarted, &alice.key, "?archived=true").await;
    assert_eq!(archived.items.len(), 1);
    assert_eq!(archived.items[0].source_event_id, old_id);
}
