//! Pay independently issued offers and verify the negotiated invoice on its issuer.
use crate::common::{
    assert_error, assert_status, counterparty::Counterparty, fixtures::unique, wait::wait_until, Auth, TestApp,
};
use reqwest::StatusCode;
use serde_json::json;
use std::time::Duration;
use swissknife_types::{Payment, PaymentExecutionState, PaymentSubmission};
use uuid::Uuid;

#[tokio::test]
async fn sends_fixed_and_amountless_bolt12_requests_and_enforces_the_payer_amount() {
    let app = TestApp::isolated("bolt12-send", &[]).await;
    let admin = app.admin_token().await;
    let account = app.create_account_with_wallet(admin, "offers-payer").await;
    let api = app.api();
    let auth = Auth::ApiKey(&account.key);
    let path = format!("/v1/me/wallets/{}/payments", account.wallet.id);
    if app.provider.starts_with("lnd") {
        let result = api
            .post(
                &format!("{path}/submissions"),
                auth,
                json!({"input":"lno1unsupported","idempotency_key":Uuid::new_v4(),"amount_msat":10000}),
            )
            .await;
        assert_error(&result, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(result.body["reason"]
            .as_str()
            .unwrap()
            .contains("does not support BOLT12"));
        return;
    }
    app.fund_onchain(admin, account.wallet.id, 100_000).await;
    let peer = Counterparty::offers_peer();
    let fixed = peer.cln_rpc(
        "offer",
        json!({"amount":"42000msat","description":unique("fixed offer")}),
    )["bolt12"]
        .as_str()
        .unwrap()
        .to_owned();
    let variable = peer.cln_rpc("offer", json!({"amount":"any","description":unique("flexible offer")}))["bolt12"]
        .as_str()
        .unwrap()
        .to_owned();
    let status_path = format!("{path}/submissions");
    let mismatch = api
        .post(
            &status_path,
            auth,
            json!({"input":fixed,"amount_msat":43000,"idempotency_key":Uuid::new_v4()}),
        )
        .await;
    assert_error(&mismatch, StatusCode::UNPROCESSABLE_ENTITY);
    let missing = api
        .post(
            &status_path,
            auth,
            json!({"input":variable,"idempotency_key":Uuid::new_v4()}),
        )
        .await;
    assert_error(&missing, StatusCode::UNPROCESSABLE_ENTITY);
    let quote = api
        .post(&format!("{path}/fee-estimate"), auth, json!({"input":fixed}))
        .await;
    assert_status(&quote, StatusCode::OK);
    assert_eq!(quote.parse::<serde_json::Value>()["amount_msat"], 42000);
    let body = json!({"input":fixed,"comment":"one approved order","idempotency_key":Uuid::new_v4()});
    let first = api.post(&status_path, auth, &body).await;
    assert_status(&first, StatusCode::ACCEPTED);
    let first: PaymentSubmission = first.parse();
    let replay: PaymentSubmission = api.post(&status_path, auth, &body).await.parse();
    assert_eq!(replay.payment.id, first.payment.id);
    wait_until(Duration::from_secs(35), "offer settles", || async {
        api.get(&first.status_url, auth)
            .await
            .parse::<PaymentSubmission>()
            .state
            == PaymentExecutionState::Settled
    })
    .await;
    let hash = first.payment.lightning.unwrap().payment_hash;
    let issued = peer.cln_rpc("listinvoices", json!({"payment_hash":hash}));
    assert_eq!(issued["invoices"][0]["status"], "paid");
    assert_eq!(issued["invoices"][0]["invreq_payer_note"], "one approved order");
    let paid = api
        .post(&path, auth, json!({"input":variable,"amount_msat":47000}))
        .await;
    assert_status(&paid, StatusCode::OK);
    assert_eq!(paid.parse::<Payment>().amount_msat, 47000);
    let source = Counterparty::for_provider("lnd_grpc");
    let negotiated = source.cln_rpc("fetchinvoice", json!({"offer":fixed,"timeout":15}));
    let paid = api.post(&path, auth, json!({"input":negotiated["invoice"]})).await;
    assert_status(&paid, StatusCode::OK);
    assert_eq!(paid.parse::<Payment>().amount_msat, 42000);
    assert_eq!(app.wallet_balance(admin, account.wallet.id).await.reserved_msat, 0);
}
