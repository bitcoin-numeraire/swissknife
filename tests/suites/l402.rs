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

#[tokio::test]
async fn a_paid_pack_survives_failure_restart_and_competing_replicas() {
    let setup = TestApp::isolated("pack", &[]).await;
    let owner = setup
        .create_account_with_wallet(setup.admin_token().await, "pack-merchant")
        .await;
    let upstream = MockServer::start().await;
    let env = [
        ("SWISSKNIFE_L402__ROOT_KEY", hex::encode([18u8; 32])),
        ("SWISSKNIFE_L402__ROUTES__DATA__WALLET_ID", owner.wallet.id.to_string()),
        (
            "SWISSKNIFE_L402__ROUTES__DATA__TARGET",
            format!("{}/read", upstream.uri()),
        ),
        ("SWISSKNIFE_L402__ROUTES__DATA__PRICE_SATS", "21".into()),
        ("SWISSKNIFE_L402__ROUTES__DATA__ACCESS_SECONDS", "600".into()),
        ("SWISSKNIFE_L402__ROUTES__DATA__REQUESTS_PER_PAYMENT", "3".into()),
    ];
    let app = setup.restart("pack-first", &env).await;
    let http = Client::new();
    let url = |app: &TestApp| format!("{}/l402/data", app.base_url);
    let challenge = http.get(url(&app)).send().await.unwrap();
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
    let preimage = Counterparty::for_provider(&app.provider).pay_for_preimage(invoice);
    let proof = format!("L402 {macaroon}:{preimage}");
    let paid = |app: &TestApp| http.get(url(app)).header(header::AUTHORIZATION, &proof).send();
    // The mock's default 404 produces a gateway error and consumes no request.
    assert_eq!(paid(&app).await.unwrap().status(), StatusCode::BAD_GATEWAY);
    Mock::given(method("GET"))
        .and(path("/read"))
        .respond_with(ResponseTemplate::new(200).set_body_string("delivery"))
        .mount(&upstream)
        .await;
    assert_eq!(paid(&app).await.unwrap().text().await.unwrap(), "delivery");
    let app = app.restart("pack-resumed", &env).await;
    let replica = app.replica("pack-replica", &env).await;
    let results = futures_util::future::join_all((0..8).map(|i| paid(if i % 2 == 0 { &app } else { &replica }))).await;
    let mut deliveries = 0;
    for response in results {
        let response = response.unwrap();
        match response.status() {
            StatusCode::OK => {
                assert_eq!(response.text().await.unwrap(), "delivery");
                deliveries += 1;
            }
            StatusCode::PAYMENT_REQUIRED => {
                assert!(response.headers().contains_key(header::WWW_AUTHENTICATE));
            }
            other => panic!("unexpected pack redemption status {other}"),
        }
    }
    assert_eq!(deliveries, 2);
    let count = upstream.received_requests().await.unwrap().len();
    assert_eq!(paid(&replica).await.unwrap().status(), StatusCode::PAYMENT_REQUIRED);
    assert_eq!(count, upstream.received_requests().await.unwrap().len());
    wait_until(Duration::from_secs(40), "one pack payment credited", || async {
        app.wallet_balance(app.admin_token().await, owner.wallet.id)
            .await
            .available_msat
            == 21_000
    })
    .await;
    replica.stop();
    let mut enlarged = env;
    enlarged[5].1 = "10".into();
    let changed = app.restart("pack-new-terms", &enlarged).await;
    assert_eq!(paid(&changed).await.unwrap().status(), StatusCode::PAYMENT_REQUIRED);
    changed.stop();
}

#[tokio::test]
async fn merchant_quotes_set_exact_invoice_amounts_and_paid_passes_survive_repricing() {
    let setup = TestApp::isolated("merchant-quote", &[]).await;
    let owner = setup
        .create_account_with_wallet(setup.admin_token().await, "quoted-merchant")
        .await;
    let quotes = MockServer::start().await;
    let upstream = MockServer::start().await;
    let env = [
        ("SWISSKNIFE_L402__ROOT_KEY", hex::encode([19u8; 32])),
        (
            "SWISSKNIFE_L402__ROUTES__REPORT__WALLET_ID",
            owner.wallet.id.to_string(),
        ),
        (
            "SWISSKNIFE_L402__ROUTES__REPORT__TARGET",
            format!("{}/read", upstream.uri()),
        ),
        ("SWISSKNIFE_L402__ROUTES__REPORT__PRICE_SATS", "60".into()),
        (
            "SWISSKNIFE_L402__ROUTES__REPORT__QUOTE_URL",
            format!("{}/report", quotes.uri()),
        ),
        ("SWISSKNIFE_L402__ROUTES__REPORT__ACCESS_SECONDS", "600".into()),
    ];
    let app = setup.restart("quoted", &env).await;
    let http = Client::new();
    let url = |app: &TestApp| format!("{}/l402/report?caller=private", app.base_url);
    let request = |app: &TestApp| {
        http.get(url(app))
            .header(header::COOKIE, "private=test")
            .header("Wallet-Key", "private-key")
    };
    for invalid in [
        ResponseTemplate::new(200).set_body_string(r#"{"price_msat":0}"#),
        ResponseTemplate::new(200).set_body_string(r#"{"price_msat":60001}"#),
        ResponseTemplate::new(200).set_body_string(r#"{"price_msat":1.5}"#),
        ResponseTemplate::new(302).insert_header("Location", format!("{}/redirected", upstream.uri())),
        ResponseTemplate::new(200).set_body_string(" ".repeat(1100)),
        ResponseTemplate::new(500),
    ] {
        quotes.reset().await;
        Mock::given(method("GET"))
            .and(path("/report"))
            .respond_with(invalid)
            .mount(&quotes)
            .await;
        let response = request(&app).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(!response.headers().contains_key(header::WWW_AUTHENTICATE));
        assert!(upstream.received_requests().await.unwrap().is_empty());
    }
    let mut original = None;
    for price in [21_345u64, 60_000] {
        quotes.reset().await;
        Mock::given(method("GET"))
            .and(path("/report"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"price_msat":price})))
            .mount(&quotes)
            .await;
        let response = request(&app)
            .header(header::AUTHORIZATION, "Bearer private")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYMENT_REQUIRED);
        let (macaroon, invoice) = response.headers()[header::WWW_AUTHENTICATE]
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
            Some(price)
        );
        let requests = quotes.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].url.query().is_none());
        for credential in ["authorization", "cookie", "wallet-key"] {
            assert!(requests[0].headers.get(credential).is_none());
        }
        if original.is_none() {
            original = Some((macaroon.to_owned(), invoice.to_owned()));
        }
    }
    let (macaroon, invoice) = original.unwrap();
    let preimage = Counterparty::for_provider(&app.provider).pay_for_preimage(&invoice);
    let proof = format!("L402 {macaroon}:{preimage}");
    wait_until(
        Duration::from_secs(40),
        "original merchant quote is credited",
        || async {
            app.wallet_balance(app.admin_token().await, owner.wallet.id)
                .await
                .available_msat
                == 21_345
        },
    )
    .await;
    let app = app.restart("quoted-redeem", &env).await;
    quotes.reset().await;
    Mock::given(method("GET"))
        .and(path("/read"))
        .respond_with(ResponseTemplate::new(200).set_body_string("paid report"))
        .mount(&upstream)
        .await;
    let response = request(&app)
        .header(header::AUTHORIZATION, &proof)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "paid report");
    assert!(quotes.received_requests().await.unwrap().is_empty());
    let mut changed = env;
    changed[4].1 = format!("{}/new-policy", quotes.uri());
    let app = app.restart("new-quote-policy", &changed).await;
    let response = request(&app)
        .header(header::AUTHORIZATION, &proof)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(quotes.received_requests().await.unwrap()[0].url.path(), "/new-policy");
    assert_eq!(upstream.received_requests().await.unwrap().len(), 1);
    app.stop();
}
