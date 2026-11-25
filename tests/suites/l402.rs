use crate::common::{counterparty::Counterparty, wait::wait_until, TestApp};
use reqwest::{header, Client, StatusCode};
use std::time::Duration;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn inspection_token(original: &str) -> String {
    let mut macaroon = libmacaroon::Macaroon::deserialize(original.as_bytes()).unwrap();
    macaroon.add_first_party_caveat("purpose=status").unwrap();
    macaroon.serialize(libmacaroon::Format::V2).unwrap()
}

async fn assert_pass_status(http: &Client, resource: &str, inspection: &str, state: &str, remaining: Option<u64>) {
    let reply = http
        .head(format!("{resource}/status"))
        .header("SwissKnife-L402-Inspection", inspection)
        .send()
        .await
        .unwrap();
    assert_eq!(
        reply.status(),
        if state == "invalid" {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::OK
        }
    );
    assert_eq!(reply.headers()["swissknife-l402-status"], state);
    assert_eq!(
        reply
            .headers()
            .get("swissknife-l402-remaining")
            .map(|v| v.to_str().unwrap().parse::<u64>().unwrap()),
        remaining
    );
    assert_eq!(reply.headers()[header::CACHE_CONTROL], "private, no-store");
    assert!(!reply.headers().contains_key(header::WWW_AUTHENTICATE));
    assert!(reply.bytes().await.unwrap().is_empty());
}

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
    // Invalid payment proofs do not consume this route's one invoice per minute.
    let denied = http
        .get(&address)
        .header(header::AUTHORIZATION, "L402 malformed")
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(denied.headers()[header::WWW_AUTHENTICATE], "L402");
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
    let inspection = inspection_token(macaroon);
    assert_pass_status(&http, &url(&app), macaroon, "invalid", None).await;
    assert_pass_status(&http, &url(&app), &inspection, "active", Some(3)).await;
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
    assert_eq!(paid(&replica).await.unwrap().text().await.unwrap(), "delivery");
    let before = upstream.received_requests().await.unwrap().len();
    for server in [&app, &replica, &app] {
        assert_pass_status(&http, &url(server), &inspection, "active", Some(1)).await;
    }
    assert_eq!(upstream.received_requests().await.unwrap().len(), before);
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
    assert_eq!(deliveries, 1);
    assert_pass_status(&http, &url(&replica), &inspection, "exhausted", Some(0)).await;
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
    assert_pass_status(&http, &url(&changed), &inspection, "inactive", None).await;
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

#[tokio::test]
async fn merchant_revenue_counts_only_settled_attributed_invoices_for_the_owned_wallet() {
    use crate::common::Auth;
    use serde_json::json;
    let setup = TestApp::isolated("merchant-revenue", &[]).await;
    let admin = setup.admin_token().await;
    let owner = setup.create_account_with_wallet(admin, "income").await;
    let other = setup.create_account_with_wallet(admin, "other-income").await;
    let env = [
        ("SWISSKNIFE_L402__ROOT_KEY", hex::encode([20u8; 32])),
        ("SWISSKNIFE_L402__ROUTES__DATA__WALLET_ID", owner.wallet.id.to_string()),
        (
            "SWISSKNIFE_L402__ROUTES__DATA__TARGET",
            "http://127.0.0.1:1/read".into(),
        ),
        ("SWISSKNIFE_L402__ROUTES__DATA__PRICE_SATS", "7".into()),
        ("SWISSKNIFE_L402__ROUTES__DATA__ACCESS_SECONDS", "600".into()),
    ];
    let app = setup.restart("revenue", &env).await;
    let api = app.api();
    let path = format!("/v1/me/wallets/{}/l402/revenue", owner.wallet.id);
    let own = Auth::ApiKey(&owner.key);
    assert_eq!(api.get(&path, Auth::None).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        api.get(&path, Auth::ApiKey(&other.key)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        api.get(&format!("{path}?from=2000-01-01&through=2001-01-01"), own)
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let payer = Counterparty::for_provider(&app.provider);
    let http = Client::new();
    for index in 0..3 {
        let response = http.get(format!("{}/l402/data", app.base_url)).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::PAYMENT_REQUIRED);
        let (_, invoice) = response.headers()[header::WWW_AUTHENTICATE]
            .to_str()
            .unwrap()
            .strip_prefix("L402 macaroon=\"")
            .unwrap()
            .strip_suffix('"')
            .unwrap()
            .split_once("\", invoice=\"")
            .unwrap();
        if index < 2 {
            payer.pay_for_preimage(invoice);
        }
    }
    // An ordinary buyer-controlled invoice description cannot impersonate a merchant sale.
    let normal = api
        .post(
            &format!("/v1/me/wallets/{}/invoices", owner.wallet.id),
            own,
            json!({"amount_msat":3000,"description":"L402 access to data","merchant_route":"data"}),
        )
        .await;
    assert_eq!(normal.status, StatusCode::OK, "{}", normal.body);
    assert!(normal.body.get("merchant_route").is_none());
    payer.pay_for_preimage(normal.body["ln_invoice"]["bolt11"].as_str().unwrap());
    wait_until(
        Duration::from_secs(40),
        "merchant and regular invoices settled",
        || async {
            app.wallet_balance(app.admin_token().await, owner.wallet.id)
                .await
                .available_msat
                == 17_000
        },
    )
    .await;
    let expected = json!([{"route":"data","settled_invoices":2,"received_msat":"14000","fee_msat":"0"}]);
    let response = api.get(&path, own).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.body);
    assert_eq!(response.body, expected);
    let empty = api
        .get(&format!("{path}?from=2000-01-01&through=2000-01-01"), own)
        .await;
    assert_eq!(empty.body, json!([]));
    let app = app.restart("revenue-offline", &[]).await;
    assert_eq!(app.api().get(&path, own).await.body, expected);
    let empty = app
        .api()
        .get(
            &format!("/v1/me/wallets/{}/l402/revenue", other.wallet.id),
            Auth::ApiKey(&other.key),
        )
        .await;
    assert_eq!(empty.body, json!([]));
    app.stop();
}

#[tokio::test]
async fn two_merchants_publish_independent_routes_and_honor_passes_during_a_sales_pause() {
    use crate::common::Auth;
    use serde_json::json;
    let setup = TestApp::isolated("merchant-catalog", &[]).await;
    let admin = setup.admin_token().await;
    let a = setup.create_account_with_wallet(admin, "publisher-a").await;
    let b = setup.create_account_with_wallet(admin, "publisher-b").await;
    let inactive = setup
        .api()
        .get(
            &format!("/v1/me/wallets/{}/l402/routes", a.wallet.id),
            Auth::ApiKey(&a.key),
        )
        .await;
    assert_eq!(inactive.status, StatusCode::OK);
    assert_eq!(inactive.body, json!({"origins":[],"routes":[]}));
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/daily"))
        .respond_with(ResponseTemplate::new(200).set_body_string("daily report"))
        .mount(&upstream)
        .await;
    let origin = format!("{}/daily?account=private", upstream.uri());
    let env = [
        ("SWISSKNIFE_L402__ROOT_KEY", hex::encode([24u8; 32])),
        ("SWISSKNIFE_L402__ORIGINS__DATA__WALLET_ID", a.wallet.id.to_string()),
        ("SWISSKNIFE_L402__ORIGINS__DATA__TARGET", origin.clone()),
        ("SWISSKNIFE_L402__ORIGINS__DATA__PRICE_SATS", "50".into()),
        ("SWISSKNIFE_L402__ORIGINS__DATA__ACCESS_SECONDS", "600".into()),
        ("SWISSKNIFE_L402__ORIGINS__ARCHIVE__WALLET_ID", a.wallet.id.to_string()),
        ("SWISSKNIFE_L402__ORIGINS__ARCHIVE__TARGET", origin.clone()),
        ("SWISSKNIFE_L402__ORIGINS__ARCHIVE__PRICE_SATS", "100".into()),
        ("SWISSKNIFE_L402__ORIGINS__ARCHIVE__ACCESS_SECONDS", "600".into()),
        ("SWISSKNIFE_L402__ORIGINS__FORECAST__WALLET_ID", b.wallet.id.to_string()),
        ("SWISSKNIFE_L402__ORIGINS__FORECAST__TARGET", origin),
        ("SWISSKNIFE_L402__ORIGINS__FORECAST__PRICE_SATS", "100".into()),
        ("SWISSKNIFE_L402__ORIGINS__FORECAST__ACCESS_SECONDS", "600".into()),
    ];
    let app = setup.restart("publish", &env).await;
    let api = app.api();
    let own_a = Auth::ApiKey(&a.key);
    let own_b = Auth::ApiKey(&b.key);
    let path_a = format!("/v1/me/wallets/{}/l402/routes", a.wallet.id);
    let path_b = format!("/v1/me/wallets/{}/l402/routes", b.wallet.id);
    let sale_a = format!("{path_a}/daily");
    let sale_b = format!("{path_b}/daily");
    let terms = |origin: &str, price: u64, enabled: bool| json!({"origin":origin,"price_msat":price.to_string(),"enabled":enabled});
    assert_eq!(api.get(&path_a, Auth::None).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(api.get(&path_a, own_b).await.status, StatusCode::NOT_FOUND);
    let delegated = api.post(
        &format!("/v1/me/wallets/{}/agent-keys", a.wallet.id),
        own_a,
        json!({"name":"merchant reader","capabilities":{"read":true,"receive":true},"expires_at":chrono::Utc::now()+chrono::Duration::hours(1)}),
    ).await.parse::<swissknife_types::IssuedAgentKey>();
    let grant = Auth::WalletKey(&delegated.key);
    assert_eq!(api.get(&path_a, grant).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        api.put(&sale_a, grant, terms("data", 1000, true)).await.status,
        StatusCode::UNAUTHORIZED
    );
    for invalid in ["UpperCase".to_string(), "x".repeat(58)] {
        assert_eq!(
            api.put(&format!("{path_a}/{invalid}"), own_a, terms("data", 1000, true))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let catalog = api.get(&path_a, own_a).await;
    assert_eq!(catalog.status, StatusCode::OK);
    assert_eq!(catalog.body["origins"].as_array().unwrap().len(), 2);
    assert!(!catalog.body.to_string().contains("private") && !catalog.body.to_string().contains("forecast"));
    assert_eq!(
        api.put(&sale_a, own_b, terms("data", 1000, true)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        api.put(&sale_a, own_a, terms("forecast", 1000, true)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        api.put(&sale_a, own_a, terms("data", 50001, true)).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut arbitrary = terms("data", 1000, true);
    arbitrary["target"] = json!("http://example.invalid");
    assert!(!api.put(&sale_a, own_a, arbitrary).await.status.is_success());
    assert_eq!(
        api.put(&sale_a, own_a, terms("data", 11111, true)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        api.put(&sale_b, own_b, terms("forecast", 32000, true)).await.status,
        StatusCode::NO_CONTENT
    );
    let race = format!("{path_a}/stable-origin");
    let (left, right) = tokio::join!(
        api.put(&race, own_a, terms("data", 3000, true)),
        api.put(&race, own_a, terms("archive", 4000, true))
    );
    let mut statuses = [left.status.as_u16(), right.status.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [204, 409]);
    let http = Client::new();
    let url = |app: &TestApp, wallet: uuid::Uuid| format!("{}/l402/w/{wallet}/daily", app.base_url);
    let initial = http.get(url(&app, a.wallet.id)).send().await.unwrap();
    assert_eq!(initial.status(), StatusCode::PAYMENT_REQUIRED);
    let (macaroon, invoice) = initial.headers()[header::WWW_AUTHENTICATE]
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
        Some(11111)
    );
    assert_eq!(
        api.put(&sale_a, own_a, terms("data", 22000, false)).await.status,
        StatusCode::NO_CONTENT
    );
    let preimage = Counterparty::for_provider(&app.provider).pay_for_preimage(invoice);
    let proof = format!("L402 {macaroon}:{preimage}");
    let app = app.restart("published-restart", &env).await;
    let inspection = inspection_token(macaroon);
    assert_pass_status(&http, &url(&app, a.wallet.id), &inspection, "active", None).await;
    assert_pass_status(&http, &url(&app, b.wallet.id), &inspection, "inactive", None).await;
    assert_eq!(
        http.get(url(&app, a.wallet.id)).send().await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    let crossed = http
        .get(url(&app, b.wallet.id))
        .header(header::AUTHORIZATION, &proof)
        .send()
        .await
        .unwrap();
    assert_eq!(crossed.status(), StatusCode::PAYMENT_REQUIRED);
    assert!(upstream.received_requests().await.unwrap().is_empty());
    let paid = http
        .get(url(&app, a.wallet.id))
        .header(header::AUTHORIZATION, &proof)
        .send()
        .await
        .unwrap();
    assert_eq!(paid.status(), StatusCode::OK);
    assert_eq!(paid.text().await.unwrap(), "daily report");
    let api = app.api();
    assert_eq!(
        api.put(&sale_a, own_a, terms("data", 22000, true)).await.status,
        StatusCode::NO_CONTENT
    );
    let next = http.get(url(&app, a.wallet.id)).send().await.unwrap();
    let invoice = next.headers()[header::WWW_AUTHENTICATE]
        .to_str()
        .unwrap()
        .split_once("invoice=\"")
        .unwrap()
        .1
        .trim_end_matches('"');
    assert_eq!(
        invoice
            .parse::<lightning_invoice::Bolt11Invoice>()
            .unwrap()
            .amount_milli_satoshis(),
        Some(22000)
    );
    wait_until(
        Duration::from_secs(40),
        "wallet sale credited to the right merchant",
        || async {
            app.wallet_balance(app.admin_token().await, a.wallet.id)
                .await
                .available_msat
                == 11111
        },
    )
    .await;
    assert_eq!(
        app.wallet_balance(app.admin_token().await, b.wallet.id)
            .await
            .available_msat,
        0
    );
    let revenue = api
        .get(&format!("/v1/me/wallets/{}/l402/revenue", a.wallet.id), own_a)
        .await;
    assert_eq!(revenue.body[0]["route"], "wallet/daily");
    assert_eq!(revenue.body[0]["received_msat"], "11111");
    app.stop();
}
