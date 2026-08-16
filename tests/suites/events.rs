//! `/v1/me/events` — authenticated, permission-gated, account-scoped durable
//! SSE delivery. The settlement test pays real invoices
//! through the matrix counterparty, so the same public behavior is exercised
//! against every configured LND and CLN transport.

use std::time::Duration;

use reqwest::{header, Response, StatusCode};
use sea_orm::{
    sea_query::{Alias, Query},
    ConnectionTrait, Database,
};

use swissknife_types::{ClientEvent, ClientEventType, Invoice, NewInvoiceRequest};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::unique;
use crate::common::wait::wait_until;
use crate::common::{app, assert_error, assert_status, Auth, TestApp};

const EVENT_TIMEOUT: Duration = Duration::from_secs(60);

struct SseMessage {
    id: String,
    event_type: String,
    payload: ClientEvent,
}

async fn invoice(app: &TestApp, key: &str, wallet_id: uuid::Uuid, amount_msat: u64) -> Invoice {
    let response = app
        .api()
        .post(
            &format!("/v1/me/wallets/{wallet_id}/invoices"),
            Auth::ApiKey(key),
            NewInvoiceRequest {
                wallet_id: None,
                amount_msat,
                description: Some(unique("event-invoice")),
                expiry: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::OK);
    response.parse::<Invoice>()
}

async fn insert_client_event(app: &TestApp, wallet_id: uuid::Uuid) {
    let connection = Database::connect(&app.database_url)
        .await
        .expect("connect to isolated event database");
    let resource_id = uuid::Uuid::new_v4();
    let statement = Query::insert()
        .into_table(Alias::new("client_event"))
        .columns([
            Alias::new("wallet_id"),
            Alias::new("event_type"),
            Alias::new("resource_id"),
            Alias::new("payload"),
        ])
        .values_panic([
            wallet_id.into(),
            ClientEventType::PaymentFailed.to_string().into(),
            resource_id.into(),
            serde_json::json!({
                "id": resource_id,
                "wallet_id": wallet_id,
                "status": "Failed"
            })
            .into(),
        ])
        .to_owned();
    let statement = connection.get_database_backend().build(&statement);
    connection
        .execute_raw(statement)
        .await
        .expect("insert durable client event");
}

fn assert_stream_headers(response: &Response) {
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream")),
        "event response must use the SSE content type"
    );
    assert_eq!(
        response
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-cache, no-transform")
    );
    assert_eq!(
        response
            .headers()
            .get("x-accel-buffering")
            .and_then(|value| value.to_str().ok()),
        Some("no")
    );
}

async fn next_event(response: &mut Response) -> SseMessage {
    tokio::time::timeout(EVENT_TIMEOUT, async {
        let mut buffered = String::new();

        loop {
            let chunk = response
                .chunk()
                .await
                .expect("read SSE response")
                .expect("SSE response ended before an event arrived");
            buffered.push_str(std::str::from_utf8(&chunk).expect("SSE is UTF-8"));
            buffered = buffered.replace("\r\n", "\n");

            while let Some(end) = buffered.find("\n\n") {
                let frame = buffered[..end].to_string();
                buffered.drain(..end + 2);

                let mut id = None;
                let mut event_type = None;
                let mut data = Vec::new();
                for line in frame.lines() {
                    if let Some(value) = line.strip_prefix("id:") {
                        id = Some(value.trim_start().to_string());
                    } else if let Some(value) = line.strip_prefix("event:") {
                        event_type = Some(value.trim_start().to_string());
                    } else if let Some(value) = line.strip_prefix("data:") {
                        data.push(value.trim_start());
                    }
                }

                if data.is_empty() {
                    continue;
                }
                let payload =
                    serde_json::from_str::<ClientEvent>(&data.join("\n")).expect("SSE data contains a client event");
                return SseMessage {
                    id: id.expect("SSE event has an id"),
                    event_type: event_type.expect("SSE event has an event name"),
                    payload,
                };
            }
        }
    })
    .await
    .expect("timed out waiting for an account event")
}

mod stream {
    use super::*;

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let response = app.api().get("/v1/me/events", Auth::None).await;

        assert_error(&response, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn requires_read_transaction_permission() {
        let app = app().await;
        let admin = app.admin_token().await;
        let account = app.create_account_with_wallet(admin, "event-permission").await;
        let key = app.account_api_key(admin, account.account.id, vec![]).await;
        let response = app.api().get("/v1/me/events", Auth::ApiKey(&key)).await;

        assert_error(&response, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rejects_a_malformed_resume_cursor() {
        let app = app().await;
        let admin = app.admin_token().await;
        let account = app.create_account_with_wallet(admin, "event-cursor").await;
        let response = app
            .api()
            .event_stream("/v1/me/events", Auth::ApiKey(&account.key), Some("not-an-event-id"))
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_a_negative_query_cursor() {
        let app = app().await;
        let admin = app.admin_token().await;
        let account = app.create_account_with_wallet(admin, "event-negative-cursor").await;
        let response = app
            .api()
            .get("/v1/me/events?after=-1", Auth::ApiKey(&account.key))
            .await;

        assert_error(&response, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn streams_and_resumes_real_invoice_settlements() {
        let app = app().await;
        let admin = app.admin_token().await;
        let account = app.create_account_with_wallet(admin, "event-settlement").await;
        let other = app.create_account_with_wallet(admin, "event-other-account").await;
        let path = "/v1/me/events";

        let first_invoice = invoice(app, &account.key, account.wallet.id, 25_000_000).await;
        let first_bolt11 = first_invoice
            .ln_invoice
            .as_ref()
            .expect("invoice has a bolt11")
            .bolt11
            .clone();
        let other_invoice = invoice(app, &other.key, other.wallet.id, 20_000_000).await;
        let other_bolt11 = other_invoice
            .ln_invoice
            .as_ref()
            .expect("invoice has a bolt11")
            .bolt11
            .clone();

        // A fresh stream starts after the current durable cursor. Paying after
        // the response is established must deliver the new settlement.
        let mut first_stream = app.api().event_stream(path, Auth::ApiKey(&account.key), None).await;
        assert_stream_headers(&first_stream);
        // This event has a lower global ID but belongs to another account. If
        // account scoping regresses it will be the first frame and fail below.
        Counterparty::for_provider(&app.provider).pay(&other_bolt11);
        Counterparty::for_provider(&app.provider).pay(&first_bolt11);

        let first = next_event(&mut first_stream).await;
        assert_eq!(first.id, first.payload.id);
        assert_eq!(first.event_type, ClientEventType::InvoicePaid.to_string());
        assert_eq!(first.payload.event_type, ClientEventType::InvoicePaid);
        assert_eq!(first.payload.wallet_id, account.wallet.id);
        assert_eq!(first.payload.resource_id, first_invoice.id);
        drop(first_stream);

        // Settle while disconnected, then reconnect with Last-Event-ID. The
        // query cursor deliberately disagrees to prove the header wins.
        let second_invoice = invoice(app, &account.key, account.wallet.id, 30_000_000).await;
        let second_bolt11 = second_invoice
            .ln_invoice
            .as_ref()
            .expect("invoice has a bolt11")
            .bolt11
            .clone();
        Counterparty::for_provider(&app.provider).pay(&second_bolt11);

        let mut resumed = app
            .api()
            .event_stream(&format!("{path}?after=0"), Auth::ApiKey(&account.key), Some(&first.id))
            .await;
        assert_stream_headers(&resumed);

        let second = next_event(&mut resumed).await;
        assert_eq!(second.id, second.payload.id);
        assert_eq!(second.event_type, ClientEventType::InvoicePaid.to_string());
        assert_eq!(second.payload.event_type, ClientEventType::InvoicePaid);
        assert_eq!(second.payload.wallet_id, account.wallet.id);
        assert_eq!(second.payload.resource_id, second_invoice.id);
        assert!(
            second.id.parse::<i32>().expect("numeric second cursor")
                > first.id.parse::<i32>().expect("numeric first cursor")
        );
    }

    #[tokio::test]
    async fn expired_replay_cursor_requires_a_rest_reset() {
        let app = TestApp::isolated(
            &unique("event-retention"),
            &[
                ("SWISSKNIFE_CLIENT_EVENTS__RETENTION", "3s".to_string()),
                ("SWISSKNIFE_CLIENT_EVENTS__CLEANUP_INTERVAL", "100ms".to_string()),
            ],
        )
        .await;
        let admin = app.admin_token().await;
        let account = app.create_account_with_wallet(admin, "event-retention").await;
        let mut stream = app
            .api()
            .event_stream("/v1/me/events", Auth::ApiKey(&account.key), None)
            .await;
        assert_stream_headers(&stream);
        // The real-settlement test above covers LND/CLN event production. This
        // test inserts at the durable-log boundary so it can exercise the
        // short retention window without racing two node listeners in parallel.
        insert_client_event(&app, account.wallet.id).await;
        let event = next_event(&mut stream).await;
        drop(stream);

        wait_until(Duration::from_secs(15), "client event cursor expires", || async {
            app.api()
                .event_stream("/v1/me/events", Auth::ApiKey(&account.key), Some(&event.id))
                .await
                .status()
                == StatusCode::CONFLICT
        })
        .await;

        // Resetting from REST means reconnecting without the stale cursor. A
        // fresh stream must remain valid even when every account event was
        // pruned and its latest retained ID is absent.
        let fresh = app
            .api()
            .event_stream("/v1/me/events", Auth::ApiKey(&account.key), None)
            .await;
        assert_stream_headers(&fresh);
    }
}
