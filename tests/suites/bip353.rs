//! Keep CI independent of public DNS. Only the node's fetchbip353 reply comes from this fixture.
//! The two real CLN nodes issue, negotiate, decode and pay invoices on regtest.
use crate::common::{assert_error, assert_status, counterparty::Counterparty, wait::wait_until, Auth, TestApp};
use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
struct Node {
    offer: Arc<RwLock<String>>,
    upstream: String,
    http: reqwest::Client,
    rune: String,
}
async fn rpc(
    State(node): State<Node>,
    Path(method): Path<String>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if method == "fetchbip353" {
        if body["address"] != "shop@wallet.test" {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"message":"DNSSEC verification failed"})),
            );
        }
        return (
            StatusCode::OK,
            Json(
                json!({"proof":"00","instructions":[{"offer":*node.offer.read().unwrap(),"offchain_amount_msat":42001}]}),
            ),
        );
    }
    if ![
        "getinfo",
        "decode",
        "offer",
        "listoffers",
        "listinvoices",
        "fetchinvoice",
        "xpay",
        "listpays",
        "listfunds",
        "listchainmoves",
        "listtransactions",
        "listpeerchannels",
    ]
    .contains(&method.as_str())
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"message":"Unsupported fixture RPC"})),
        );
    }
    let res = node
        .http
        .post(format!("{}/v1/{method}", node.upstream))
        .header("Rune", &node.rune)
        .json(&body)
        .send()
        .await
        .unwrap();
    (res.status(), Json(res.json().await.unwrap()))
}

#[tokio::test]
async fn pays_verified_names_and_refuses_changes_after_review() {
    if std::env::var("SWISSKNIFE_ITEST_PROVIDER").as_deref() != Ok("cln_rest") {
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/itest/runtime/cln");
    let cert = reqwest::Certificate::from_pem(&std::fs::read(root.join("regtest/ca.pem")).unwrap()).unwrap();
    let peer = Counterparty::offers_peer();
    let issue = || {
        peer.cln_rpc(
            "offer",
            json!({"amount":"any","description":Uuid::new_v4().to_string()}),
        )["bolt12"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let original = issue();
    let offer = Arc::new(RwLock::new(original.clone()));
    let node = Node {
        offer: offer.clone(),
        upstream: format!(
            "https://127.0.0.1:{}",
            std::env::var("SWISSKNIFE_ITEST_CLN_REST_PORT").unwrap_or_else(|_| "3010".into())
        ),
        http: reqwest::Client::builder()
            .tls_certs_only([cert])
            .danger_accept_invalid_hostnames(true)
            .build()
            .unwrap(),
        rune: std::fs::read_to_string(root.join("rune")).unwrap().trim().into(),
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let fixture = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/v1/{method}", post(rpc)).with_state(node),
        )
        .await
        .unwrap()
    });
    let app = TestApp::isolated("bip353", &[("SWISSKNIFE_CLN_REST_CONFIG__ENDPOINT", url)]).await;
    let admin = app.admin_token().await;
    let a = app.create_account_with_wallet(admin, "hrn-payer").await;
    let auth = Auth::ApiKey(&a.key);
    let api = app.api();
    let path = format!("/v1/me/wallets/{}", a.wallet.id);
    let own = api
        .post(
            &format!("{path}/offers"),
            auth,
            json!({"idempotency_key":Uuid::new_v4(),"description":"fund payer","amount_msat":100000}),
        )
        .await;
    assert_status(&own, StatusCode::OK);
    let invoice = peer.cln_rpc("fetchinvoice", json!({"offer":own.body["bolt12"],"timeout":15}));
    peer.cln_rpc("xpay", json!({"invstring":invoice["invoice"]}));
    wait_until(Duration::from_secs(30), "payer funded", || async {
        app.wallet_balance(admin, a.wallet.id).await.available_msat == 100000
    })
    .await;
    let quote_url = format!("{path}/payments/fee-estimate");
    let pay_url = format!("{path}/payments/submissions");
    let input = "₿Shop@Wallet.TEST";
    let quote = api.post(&quote_url, auth, json!({"input":input})).await;
    assert_status(&quote, StatusCode::OK);
    assert_eq!(quote.body["amount_msat"], 42001);
    let pinned = quote.body["resolved_offer_id"].as_str().unwrap().to_owned();
    let request = json!({"input":input,"amount_msat":42001,"comment":"name receipt","expected_offer_id":pinned,"idempotency_key":Uuid::new_v4()});
    *offer.write().unwrap() = issue();
    let changed = api.post(&pay_url, auth, &request).await;
    assert_error(&changed, StatusCode::CONFLICT);
    assert_eq!(changed.body["code"], "payment_target_changed");
    assert_error(
        &api.post(&quote_url, auth, json!({"input":"₿missing@wallet.test"}))
            .await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_error(
        &api.post(&quote_url, auth, json!({"input":input,"amount_msat":42000}))
            .await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_eq!(app.wallet_balance(admin, a.wallet.id).await.available_msat, 100000);
    *offer.write().unwrap() = original;
    let accepted = api.post(&pay_url, auth, &request).await;
    assert_status(&accepted, StatusCode::ACCEPTED);
    let url = accepted.body["status_url"].as_str().unwrap();
    wait_until(Duration::from_secs(30), "BIP353 settlement", || async {
        api.get(url, auth).await.body["state"] == "settled"
    })
    .await;
    let payment = api.get(url, auth).await.body["payment"].clone();
    assert_eq!(payment["lightning"]["ln_address"], "₿shop@wallet.test");
    let paid = peer.cln_rpc(
        "listinvoices",
        json!({"payment_hash":payment["lightning"]["payment_hash"]}),
    )["invoices"][0]
        .clone();
    assert_eq!(paid["status"], "paid");
    assert_eq!(paid["amount_received_msat"], 42001);
    let decoded = peer.cln_rpc("decode", json!({"string":paid["bolt12"]}));
    assert_eq!(
        decoded["invreq_bip_353_name"],
        json!({"name":"shop","domain":"wallet.test"})
    );
    *offer.write().unwrap() = String::new();
    assert_eq!(
        api.post(&pay_url, auth, request).await.body["payment"]["id"],
        payment["id"]
    );
    fixture.abort();
}
