//! `/v1/payments` — admin payment management, permission-gated (`*:transaction`).
//! The happy LN-routed send lives in the lightning suite; here we cover the
//! input/validation 422s, an instance-internal bolt11 settlement between two
//! wallets, and CRUD. Each test uses its own wallet(s) for balance isolation.

use reqwest::StatusCode;

use swissknife_types::{Invoice, Ledger, NewInvoiceRequest, Payment, PaymentStatus, SendPaymentRequest};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth, TestApp};

/// Generate an invoice for `wallet_id` and return its bolt11.
async fn invoice_bolt11(app: &TestApp, token: &str, wallet_id: uuid::Uuid, amount_msat: u64) -> String {
    let res = app
        .api()
        .post(
            "/v1/invoices",
            Auth::Bearer(token),
            NewInvoiceRequest {
                wallet_id: Some(wallet_id),
                amount_msat,
                description: None,
                expiry: None,
            },
        )
        .await;
    assert_status(&res, StatusCode::OK);
    res.parse::<Invoice>().ln_invoice.expect("a bolt11 invoice").bolt11
}

fn pay(wallet_id: uuid::Uuid, input: String) -> SendPaymentRequest {
    SendPaymentRequest {
        wallet_id: Some(wallet_id),
        input,
        amount_msat: None,
        comment: None,
    }
}

mod send {
    use super::*;

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app
            .api()
            .post("/v1/payments", Auth::None, pay(uuid::Uuid::new_v4(), "x".to_string()))
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_an_unparseable_input() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "pay-badinput").await;

        // Not a bolt11/LNURL/LN-address: parsing fails inside the use case (422).
        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, "notapaymentinput".to_string()),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn rejects_paying_your_own_invoice() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "pay-self").await;
        let bolt11 = invoice_bolt11(app, token, wallet.id, 50_000_000).await;

        // Paying an invoice your own wallet issued is rejected before any reserve.
        let res = app
            .api()
            .post("/v1/payments", Auth::Bearer(token), pay(wallet.id, bolt11))
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn rejects_when_funds_are_insufficient() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "pay-broke").await; // zero balance

        // A real external invoice the wallet cannot afford: the reserve fails (422).
        let bolt11 = Counterparty::for_provider(&app.provider).invoice(50_000_000, &unique("cp-inv"));
        let res = app
            .api()
            .post("/v1/payments", Auth::Bearer(token), pay(wallet.id, bolt11))
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

mod internal {
    use super::*;

    #[tokio::test]
    async fn settles_a_bolt11_between_two_wallets() {
        let app = app().await;
        let token = app.admin_token().await;
        let payer = app.create_wallet(token, "pay-internal-src").await;
        let payee = app.create_wallet(token, "pay-internal-dst").await;

        app.fund_onchain(token, payer.id, 200_000).await;
        let payer_before = app.wallet_balance(token, payer.id).await.available_msat;
        let payee_before = app.wallet_balance(token, payee.id).await.available_msat;

        // Paying a bolt11 the instance itself issued for another wallet settles
        // internally (no LN routing), synchronously, with no fee.
        let amount_msat = 50_000_000i64;
        let bolt11 = invoice_bolt11(app, token, payee.id, amount_msat as u64).await;
        let res = app
            .api()
            .post("/v1/payments", Auth::Bearer(token), pay(payer.id, bolt11))
            .await;
        assert_status(&res, StatusCode::OK);
        let payment = res.parse::<Payment>();
        assert_eq!(payment.status, PaymentStatus::Settled);
        assert_eq!(payment.ledger, Ledger::Internal);
        assert_eq!(payment.amount_msat, amount_msat as u64);
        assert!(payment.internal.is_some(), "internal payment details are populated");

        // The payer is debited and the payee credited by exactly the amount.
        assert_eq!(
            app.wallet_balance(token, payer.id).await.available_msat,
            payer_before - amount_msat,
            "payer debited by the amount (internal payments are fee-less)"
        );
        assert_eq!(
            app.wallet_balance(token, payee.id).await.available_msat,
            payee_before + amount_msat,
            "payee credited by the amount"
        );

        // CRUD on the resulting payment.
        let got = app
            .api()
            .get(&format!("/v1/payments/{}", payment.id), Auth::Bearer(token))
            .await;
        assert_status(&got, StatusCode::OK);
        assert_eq!(got.parse::<Payment>().id, payment.id);

        let list = app
            .api()
            .get(&format!("/v1/payments?wallet_id={}", payer.id), Auth::Bearer(token))
            .await;
        assert_status(&list, StatusCode::OK);
        assert!(
            list.parse::<Vec<Payment>>().iter().any(|p| p.id == payment.id),
            "the payment is listed for the payer wallet"
        );

        let del = app
            .api()
            .delete(&format!("/v1/payments/{}", payment.id), Auth::Bearer(token))
            .await;
        assert_status(&del, StatusCode::OK);
        let gone = app
            .api()
            .get(&format!("/v1/payments/{}", payment.id), Auth::Bearer(token))
            .await;
        assert_error(&gone, StatusCode::NOT_FOUND);
    }
}

mod manage {
    use super::*;

    #[tokio::test]
    async fn unknown_id_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .get(&format!("/v1/payments/{}", uuid::Uuid::new_v4()), Auth::Bearer(token))
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }
}
