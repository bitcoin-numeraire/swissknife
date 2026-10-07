//! Account-owned webhook lifecycle, account boundaries, and durable fan-out.

use std::time::Duration;

use reqwest::StatusCode;
use serde_json::json;
use swissknife_types::{
    CreatedWebhookSubscription, Invoice, Permission, WebhookDelivery, WebhookDeliveryStatus, WebhookSubscription,
};

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
    assert_status(&app.api().get(&item, auth).await, StatusCode::OK);
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
    let reader = app
        .api_key(app.admin_token().await, vec![Permission::ReadWebhook])
        .await;
    let admin_history = app
        .api()
        .get(
            &format!("/v1/webhooks/{}/deliveries", created.subscription.id),
            Auth::ApiKey(&reader),
        )
        .await;
    assert_status(&admin_history, StatusCode::OK);
    assert_eq!(admin_history.body, response.body);
}

mod administrative_access {
    use super::*;

    #[tokio::test]
    async fn webhook_permissions_allow_managing_another_accounts_full_lifecycle() {
        let app = app().await;
        let owner = ordinary_account(app, "webhook-admin-owner").await;
        let admin = app.admin_token().await;
        let key = app
            .api_key(admin, vec![Permission::ReadWebhook, Permission::WriteWebhook])
            .await;
        let auth = Auth::ApiKey(&key);
        let request = json!({
            "wallet_id": owner.wallet.id,
            "url": "https://example.com/admin-webhook",
            "event_types": ["invoice.paid", "payment.failed"]
        });
        let response = app.api().post("/v1/webhooks", auth, &request).await;
        assert_status(&response, StatusCode::CREATED);
        let created = response.parse::<CreatedWebhookSubscription>();
        assert_eq!(created.subscription.account_id, owner.account.id);
        assert_eq!(created.subscription.wallet_id, owner.wallet.id);
        assert!(!created.signing_secret.is_empty());
        let item = format!("/v1/webhooks/{}", created.subscription.id);
        let owned_item = format!(
            "/v1/me/wallets/{}/webhooks/{}",
            owner.wallet.id, created.subscription.id
        );
        for (path, credential) in [(&item, auth), (&owned_item, Auth::ApiKey(&owner.key))] {
            let got = app.api().get(path, credential).await;
            assert_status(&got, StatusCode::OK);
            assert_eq!(got.parse::<WebhookSubscription>().id, created.subscription.id);
            assert!(got.body.get("signing_secret").is_none());
        }
        assert_error(
            &app.api().post("/v1/webhooks", auth, request).await,
            StatusCode::CONFLICT,
        );
        let listed = app
            .api()
            .get(&format!("/v1/webhooks?account_id={}", owner.account.id), auth)
            .await;
        assert_status(&listed, StatusCode::OK);
        assert_eq!(listed.parse::<Vec<WebhookSubscription>>().len(), 1);
        assert!(listed.body[0].get("signing_secret").is_none());
        let updated = app
            .api()
            .put(
                &item,
                auth,
                json!({
                    "url": "https://example.com/updated-admin-webhook",
                    "event_types": ["payment.settled"], "active": false
                }),
            )
            .await;
        assert_status(&updated, StatusCode::OK);
        assert_eq!(updated.body["url"], "https://example.com/updated-admin-webhook");
        assert_eq!(updated.body["event_types"], json!(["payment.settled"]));
        assert_eq!(updated.body["active"], false);
        assert!(updated.body.get("signing_secret").is_none());
        let rotated = app.api().post(&format!("{item}/rotate-secret"), auth, json!({})).await;
        assert_status(&rotated, StatusCode::OK);
        assert_ne!(rotated.body["signing_secret"], created.signing_secret);
        let history = app.api().get(&format!("{item}/deliveries"), auth).await;
        assert_status(&history, StatusCode::OK);
        assert_eq!(history.body, json!([]));
        let resumed = app.api().put(&item, auth, json!({"active": true})).await;
        assert_status(&resumed, StatusCode::OK);
        assert_eq!(resumed.body["active"], true);
        assert_status(&app.api().delete(&item, auth).await, StatusCode::NO_CONTENT);
        assert_error(&app.api().get(&item, auth).await, StatusCode::NOT_FOUND);
        assert_error(
            &app.api().get(&owned_item, Auth::ApiKey(&owner.key)).await,
            StatusCode::NOT_FOUND,
        );
    }

    #[tokio::test]
    async fn read_and_write_permissions_are_independent() {
        let app = app().await;
        let owner = ordinary_account(app, "webhook-permission-owner").await;
        let admin = app.admin_token().await;
        let reader = app.api_key(admin, vec![Permission::ReadWebhook]).await;
        let writer = app.api_key(admin, vec![Permission::WriteWebhook]).await;
        let request = json!({"wallet_id": owner.wallet.id, "url": "https://example.com/permissions", "event_types": ["invoice.paid"]});
        let created = app.api().post("/v1/webhooks", Auth::ApiKey(&writer), &request).await;
        assert_status(&created, StatusCode::CREATED);
        let id = created.parse::<CreatedWebhookSubscription>().subscription.id;
        let item = format!("/v1/webhooks/{id}");
        for path in ["/v1/webhooks", &item, &format!("{item}/deliveries")] {
            assert_status(&app.api().get(path, Auth::ApiKey(&reader)).await, StatusCode::OK);
            assert_error(&app.api().get(path, Auth::ApiKey(&writer)).await, StatusCode::FORBIDDEN);
        }
        let read_auth = Auth::ApiKey(&reader);
        assert_error(
            &app.api().post("/v1/webhooks", read_auth, request).await,
            StatusCode::FORBIDDEN,
        );
        assert_error(
            &app.api().put(&item, read_auth, json!({"active": false})).await,
            StatusCode::FORBIDDEN,
        );
        assert_error(
            &app.api()
                .post(&format!("{item}/rotate-secret"), read_auth, json!({}))
                .await,
            StatusCode::FORBIDDEN,
        );
        assert_error(&app.api().delete(&item, read_auth).await, StatusCode::FORBIDDEN);
        let write_auth = Auth::ApiKey(&writer);
        assert_status(
            &app.api().put(&item, write_auth, json!({"active": false})).await,
            StatusCode::OK,
        );
        assert_status(
            &app.api()
                .post(&format!("{item}/rotate-secret"), write_auth, json!({}))
                .await,
            StatusCode::OK,
        );
        assert_status(&app.api().delete(&item, write_auth).await, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn every_admin_route_requires_its_webhook_permission_and_authentication() {
        let app = app().await;
        let owner = ordinary_account(app, "webhook-no-admin-scope").await;
        let unrelated = app
            .api_key(
                app.admin_token().await,
                vec![Permission::ReadWallet, Permission::WriteWallet],
            )
            .await;
        let item = format!("/v1/webhooks/{}", uuid::Uuid::new_v4());
        for (auth, status) in [
            (Auth::None, StatusCode::UNAUTHORIZED),
            (Auth::ApiKey(&owner.key), StatusCode::FORBIDDEN),
            (Auth::ApiKey(&unrelated), StatusCode::FORBIDDEN),
        ] {
            for path in ["/v1/webhooks", &item, &format!("{item}/deliveries")] {
                assert_error(&app.api().get(path, auth).await, status);
            }
            assert_error(&app.api().post("/v1/webhooks", auth, json!({"wallet_id": owner.wallet.id, "url": "https://example.com/denied", "event_types": ["invoice.paid"]})).await, status);
            assert_error(&app.api().put(&item, auth, json!({"active": false})).await, status);
            assert_error(
                &app.api().post(&format!("{item}/rotate-secret"), auth, json!({})).await,
                status,
            );
            assert_error(&app.api().delete(&item, auth).await, status);
        }
    }

    #[tokio::test]
    async fn admin_creation_requires_an_existing_explicit_wallet() {
        let app = app().await;
        let admin = Auth::Bearer(app.admin_token().await);
        let request = json!({"url": "https://example.com/missing-wallet", "event_types": ["invoice.paid"]});
        assert_error(
            &app.api().post("/v1/webhooks", admin, &request).await,
            StatusCode::BAD_REQUEST,
        );
        let mut request = request;
        request["wallet_id"] = json!(uuid::Uuid::new_v4());
        assert_error(
            &app.api().post("/v1/webhooks", admin, request).await,
            StatusCode::NOT_FOUND,
        );
        let item = format!("/v1/webhooks/{}", uuid::Uuid::new_v4());
        assert_error(&app.api().get(&item, admin).await, StatusCode::NOT_FOUND);
        assert_error(
            &app.api().put(&item, admin, json!({"active": false})).await,
            StatusCode::NOT_FOUND,
        );
        assert_error(
            &app.api().post(&format!("{item}/rotate-secret"), admin, json!({})).await,
            StatusCode::NOT_FOUND,
        );
        assert_error(
            &app.api().get(&format!("{item}/deliveries"), admin).await,
            StatusCode::NOT_FOUND,
        );
        assert_error(&app.api().delete(&item, admin).await, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn account_routes_keep_ownership_even_with_admin_permissions_and_forged_filters() {
        let app = app().await;
        let admin = app.admin_token().await;
        let owner = ordinary_account(app, "webhook-owned-filter").await;
        let other = ordinary_account(app, "webhook-other-filter").await;
        let key = app
            .account_api_key(
                admin,
                owner.account.id,
                vec![Permission::ReadWebhook, Permission::WriteWebhook],
            )
            .await;
        let auth = Auth::ApiKey(&key);
        let path = format!("/v1/me/wallets/{}/webhooks", owner.wallet.id);
        let request = json!({"wallet_id": other.wallet.id, "url": "https://example.com/owned-filter", "event_types": ["invoice.paid"]});
        let own = app.api().post(&path, auth, &request).await;
        assert_status(&own, StatusCode::CREATED);
        let own = own.parse::<CreatedWebhookSubscription>();
        assert_eq!(
            own.subscription.wallet_id, owner.wallet.id,
            "path overrides request wallet"
        );
        let other_path = format!("/v1/me/wallets/{}/webhooks", other.wallet.id);
        let other_sub = app
            .api()
            .post(&other_path, Auth::ApiKey(&other.key), request)
            .await
            .parse::<CreatedWebhookSubscription>();
        for path in [
            format!("/v1/me/webhooks?account_id={}", other.account.id),
            format!("{path}?account_id={}&wallet_id={}", other.account.id, other.wallet.id),
        ] {
            let response = app.api().get(&path, auth).await;
            assert_status(&response, StatusCode::OK);
            let listed = response.parse::<Vec<WebhookSubscription>>();
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].id, own.subscription.id);
        }
        for item in [
            format!("{other_path}/{}", other_sub.subscription.id),
            format!("{path}/{}", other_sub.subscription.id),
        ] {
            assert_error(&app.api().get(&item, auth).await, StatusCode::NOT_FOUND);
            assert_error(
                &app.api().put(&item, auth, json!({"active": false})).await,
                StatusCode::NOT_FOUND,
            );
            assert_error(
                &app.api().post(&format!("{item}/rotate-secret"), auth, json!({})).await,
                StatusCode::NOT_FOUND,
            );
            assert_error(
                &app.api().get(&format!("{item}/deliveries"), auth).await,
                StatusCode::NOT_FOUND,
            );
            assert_error(&app.api().delete(&item, auth).await, StatusCode::NOT_FOUND);
        }
        assert_error(&app.api().get(&other_path, auth).await, StatusCode::NOT_FOUND);
        assert_error(
            &app.api()
                .post(
                    &other_path,
                    auth,
                    json!({"url":"https://example.com/forbidden", "event_types":["invoice.paid"]}),
                )
                .await,
            StatusCode::NOT_FOUND,
        );
    }

    #[tokio::test]
    async fn list_filters_select_accounts_wallets_ids_states_and_pages() {
        let app = app().await;
        let owner = ordinary_account(app, "webhook-list-owner").await;
        let other = ordinary_account(app, "webhook-list-other").await;
        let admin = Auth::Bearer(app.admin_token().await);
        let mut ids = Vec::new();
        for (wallet_id, name) in [
            (owner.wallet.id, "first"),
            (owner.wallet.id, "second"),
            (other.wallet.id, "other"),
        ] {
            let response = app.api().post("/v1/webhooks", admin, json!({"wallet_id": wallet_id, "url": format!("https://example.com/{name}"), "event_types":["invoice.paid"]})).await;
            assert_status(&response, StatusCode::CREATED);
            ids.push(response.parse::<CreatedWebhookSubscription>().subscription.id);
        }
        assert_status(
            &app.api()
                .put(&format!("/v1/webhooks/{}", ids[1]), admin, json!({"active":false}))
                .await,
            StatusCode::OK,
        );
        for (query, expected) in [
            (format!("account_id={}&active=false", owner.account.id), vec![ids[1]]),
            (format!("wallet_id={}&active=true", owner.wallet.id), vec![ids[0]]),
            (format!("ids={}", ids[2]), vec![ids[2]]),
            (
                format!("account_id={}&wallet_id={}", owner.account.id, other.wallet.id),
                vec![],
            ),
        ] {
            let response = app.api().get(&format!("/v1/webhooks?{query}"), admin).await;
            assert_status(&response, StatusCode::OK);
            assert_eq!(
                response
                    .parse::<Vec<WebhookSubscription>>()
                    .iter()
                    .map(|s| s.id)
                    .collect::<Vec<_>>(),
                expected
            );
        }
        let mut paged = Vec::new();
        for offset in 0..2 {
            let response = app
                .api()
                .get(
                    &format!("/v1/webhooks?wallet_id={}&limit=1&offset={offset}", owner.wallet.id),
                    admin,
                )
                .await;
            assert_status(&response, StatusCode::OK);
            let page = response.parse::<Vec<WebhookSubscription>>();
            assert_eq!(page.len(), 1);
            paged.push(page[0].id);
        }
        assert_ne!(paged[0], paged[1]);
        assert!(paged.iter().all(|id| ids[..2].contains(id)));
    }
}
