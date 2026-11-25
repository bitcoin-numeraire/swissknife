use crate::common::{counterparty::Counterparty, wait::wait_until, TestApp};
use reqwest::{header, Client, StatusCode};
use std::time::Duration;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn a_real_invoice_unlocks_only_the_selected_merchant_resource() {
    let app = TestApp::isolated("l402", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "merchant").await;
    let other = app.create_account_with_wallet(admin, "other").await;
    let http = Client::new();
    assert_eq!(
        http.get(format!("{}/l402/report", app.base_url))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let upstream = MockServer::start().await;
    let target = format!("{}/export", upstream.uri());
    let env = [
        ("SWISSKNIFE_L402__ROOT_KEY", hex::encode([17u8; 32])),
        (
            "SWISSKNIFE_L402__ROUTES__REPORT__WALLET_ID",
            owner.wallet.id.to_string(),
        ),
        ("SWISSKNIFE_L402__ROUTES__REPORT__TARGET", target.clone()),
        ("SWISSKNIFE_L402__ROUTES__REPORT__PRICE_SATS", "42".into()),
        ("SWISSKNIFE_L402__ROUTES__REPORT__ACCESS_SECONDS", "600".into()),
        ("SWISSKNIFE_L402__ROUTES__REPORT__CHALLENGES_PER_MINUTE", "1".into()),
        ("SWISSKNIFE_L402__ROUTES__OTHER__WALLET_ID", other.wallet.id.to_string()),
        ("SWISSKNIFE_L402__ROUTES__OTHER__TARGET", target),
        ("SWISSKNIFE_L402__ROUTES__OTHER__PRICE_SATS", "42".into()),
        ("SWISSKNIFE_L402__ROUTES__OTHER__ACCESS_SECONDS", "600".into()),
    ];
    let app = app.restart("configured", &env).await;
    let address = format!("{}/l402/report", app.base_url);
    assert_eq!(
        http.head(&address).send().await.unwrap().status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        http.post(&address).send().await.unwrap().status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    let challenge = http.get(&address).send().await.unwrap();
    assert_eq!(challenge.status(), StatusCode::PAYMENT_REQUIRED);
    let (macaroon, invoice) = challenge.headers()[header::WWW_AUTHENTICATE]
        .to_str()
        .unwrap()
        .strip_prefix("L402 macaroon=\"")
        .unwrap()
        .strip_suffix('"')
        .unwrap()
        .split_once("\", invoice=\"")
        .unwrap();
    assert_eq!(
        invoice
            .parse::<lightning_invoice::Bolt11Invoice>()
            .unwrap()
            .amount_milli_satoshis(),
        Some(42_000)
    );
    assert_eq!(
        http.get(&address).send().await.unwrap().status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert!(upstream.received_requests().await.unwrap().is_empty());
    let preimage = Counterparty::for_provider(&app.provider).pay_for_preimage(invoice);
    let proof = format!("L402 {macaroon}:{preimage}");
    wait_until(Duration::from_secs(40), "merchant wallet credited", || async {
        app.wallet_balance(app.admin_token().await, owner.wallet.id)
            .await
            .available_msat
            == 42_000
    })
    .await;
    let app = app.restart("paid", &env).await;
    let cross = http
        .get(format!("{}/l402/other", app.base_url))
        .header(header::AUTHORIZATION, &proof)
        .send()
        .await
        .unwrap();
    assert_eq!(cross.status(), StatusCode::PAYMENT_REQUIRED);
    assert!(upstream.received_requests().await.unwrap().is_empty());
    let address = format!("{}/l402/report?caller_query=ignored", app.base_url);
    let request = || {
        http.get(&address)
            .header(header::AUTHORIZATION, &proof)
            .header(header::COOKIE, "private=test")
            .header("Wallet-Key", "private-wallet-key")
    };
    Mock::given(method("GET"))
        .and(path("/export"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("merchant report")
                .insert_header("Set-Cookie", "upstream=private"),
        )
        .mount(&upstream)
        .await;
    for _ in 0..2 {
        let paid = request().send().await.unwrap();
        assert_eq!(paid.status(), StatusCode::OK);
        assert_eq!(paid.headers()[header::CACHE_CONTROL], "private, no-store");
        assert_eq!(paid.headers()[header::CONTENT_DISPOSITION], "attachment");
        assert!(paid.headers().get(header::SET_COOKIE).is_none());
        assert_eq!(paid.text().await.unwrap(), "merchant report");
    }
    let requests = upstream.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for req in requests {
        assert!(req.url.query().is_none());
        for name in ["authorization", "cookie", "wallet-key"] {
            assert!(req.headers.get(name).is_none());
        }
    }
    upstream.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/elsewhere"))
        .mount(&upstream)
        .await;
    assert_eq!(request().send().await.unwrap().status(), StatusCode::BAD_GATEWAY);
    assert_eq!(upstream.received_requests().await.unwrap().len(), 1);
    upstream.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0; 2 * 1024 * 1024 + 1]))
        .mount(&upstream)
        .await;
    assert_eq!(request().send().await.unwrap().status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        app.wallet_balance(app.admin_token().await, owner.wallet.id)
            .await
            .available_msat,
        42_000
    );
    assert_eq!(
        app.wallet_balance(app.admin_token().await, other.wallet.id)
            .await
            .available_msat,
        0
    );
    let mut changed = env;
    changed[3].1 = "43".into();
    let app = app.restart("repriced", &changed).await;
    let repriced = http
        .get(format!("{}/l402/report", app.base_url))
        .header(header::AUTHORIZATION, &proof)
        .send()
        .await
        .unwrap();
    assert_eq!(repriced.status(), StatusCode::PAYMENT_REQUIRED);
    app.stop();
}
