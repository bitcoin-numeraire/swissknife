//! `/v1/invoices` — admin invoice management, permission-gated (`*:transaction`).
//! Generating an invoice reaches the LN node, so these run across the provider
//! matrix. Each test uses its own wallet so list/filter counts stay deterministic
//! on the shared instance.

use reqwest::StatusCode;

use swissknife_types::{Invoice, InvoiceStatus, NewInvoiceRequest};

use crate::common::{app, assert_error, assert_status, Auth, TestApp};

/// Generate a Lightning invoice for `wallet_id` and return it.
async fn new_invoice(app: &TestApp, token: &str, wallet_id: uuid::Uuid, amount_msat: u64) -> Invoice {
    let res = app
        .api()
        .post(
            "/v1/invoices",
            Auth::Bearer(token),
            NewInvoiceRequest {
                wallet_id: Some(wallet_id),
                amount_msat,
                description: Some("itest invoice".to_string()),
                expiry: None,
            },
        )
        .await;
    assert_status(&res, StatusCode::OK);
    res.parse()
}

mod generate {
    use super::*;

    #[tokio::test]
    async fn creates_a_lightning_invoice() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "inv-create").await;

        let invoice = new_invoice(app, token, wallet.id, 21_000).await;

        assert_eq!(invoice.wallet_id, wallet.id);
        assert_eq!(invoice.amount_msat, Some(21_000));
        assert_eq!(invoice.status, InvoiceStatus::Pending);
        let ln = invoice.ln_invoice.expect("a lightning invoice carries bolt11 details");
        assert!(ln.bolt11.starts_with("lnbcrt"), "regtest bolt11, got {}", ln.bolt11);
        assert!(!ln.payment_hash.is_empty());
    }

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app
            .api()
            .post(
                "/v1/invoices",
                Auth::None,
                NewInvoiceRequest {
                    wallet_id: None,
                    amount_msat: 1_000,
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_a_body_without_an_amount() {
        let app = app().await;
        let token = app.admin_token().await;
        // A missing required field fails JSON deserialization, which this API
        // maps to 400 Malformed. (422 is reserved for use-case validation.)
        let res = app
            .api()
            .post("/v1/invoices", Auth::Bearer(token), serde_json::json!({}))
            .await;
        assert_error(&res, StatusCode::BAD_REQUEST);
    }
}

mod query {
    use super::*;

    #[tokio::test]
    async fn get_by_id() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "inv-get").await;
        let invoice = new_invoice(app, token, wallet.id, 5_000).await;

        let got = app
            .api()
            .get(&format!("/v1/invoices/{}", invoice.id), Auth::Bearer(token))
            .await;
        assert_status(&got, StatusCode::OK);
        assert_eq!(got.parse::<Invoice>().id, invoice.id);
    }

    #[tokio::test]
    async fn unknown_id_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .get(&format!("/v1/invoices/{}", uuid::Uuid::new_v4()), Auth::Bearer(token))
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn filters_by_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "inv-filter").await;
        let invoice = new_invoice(app, token, wallet.id, 7_000).await;

        let res = app
            .api()
            .get(&format!("/v1/invoices?wallet_id={}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&res, StatusCode::OK);
        let invoices = res.parse::<Vec<Invoice>>();
        assert_eq!(invoices.len(), 1, "the fresh wallet has exactly one invoice");
        assert_eq!(invoices[0].id, invoice.id);
    }

    #[tokio::test]
    async fn filters_by_pending_status() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "inv-pending").await;
        let invoice = new_invoice(app, token, wallet.id, 9_000).await;

        // A freshly generated invoice is Pending (future expiry, unpaid), so the
        // status filter must return it.
        let res = app
            .api()
            .get(
                &format!("/v1/invoices?wallet_id={}&status=Pending", wallet.id),
                Auth::Bearer(token),
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let invoices = res.parse::<Vec<Invoice>>();
        assert_eq!(invoices.len(), 1, "the pending invoice is returned by the status filter");
        assert_eq!(invoices[0].id, invoice.id);
        assert_eq!(invoices[0].status, InvoiceStatus::Pending);
    }

    #[tokio::test]
    async fn list_respects_the_limit() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "inv-limit").await;
        for _ in 0..3 {
            new_invoice(app, token, wallet.id, 1_000).await;
        }

        let res = app
            .api()
            .get(&format!("/v1/invoices?wallet_id={}&limit=2", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(res.parse::<Vec<Invoice>>().len(), 2, "limit caps the result count");
    }
}

mod delete {
    use super::*;

    #[tokio::test]
    async fn deletes_a_single_invoice() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "inv-del").await;
        let invoice = new_invoice(app, token, wallet.id, 3_000).await;

        let del = app
            .api()
            .delete(&format!("/v1/invoices/{}", invoice.id), Auth::Bearer(token))
            .await;
        assert_status(&del, StatusCode::OK);

        let gone = app
            .api()
            .get(&format!("/v1/invoices/{}", invoice.id), Auth::Bearer(token))
            .await;
        assert_error(&gone, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn bulk_deletes_by_wallet_filter() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "inv-bulk").await;
        new_invoice(app, token, wallet.id, 1_000).await;
        new_invoice(app, token, wallet.id, 2_000).await;

        let del = app
            .api()
            .delete(&format!("/v1/invoices?wallet_id={}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&del, StatusCode::OK);
        assert_eq!(del.parse::<u64>(), 2, "both invoices are deleted");

        let list = app
            .api()
            .get(&format!("/v1/invoices?wallet_id={}", wallet.id), Auth::Bearer(token))
            .await;
        assert_eq!(list.parse::<Vec<Invoice>>().len(), 0, "no invoices remain");
    }
}
