use crate::common::{assert_status, counterparty::Counterparty, fixtures::unique, Auth, TestApp};
use chrono::{Duration as ChronoDuration, Utc};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::{process::Stdio, time::Duration};
use swissknife_types::IssuedAgentKey;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};
use uuid::Uuid;

struct StdioWallet {
    child: Child,
    input: ChildStdin,
    output: Lines<BufReader<ChildStdout>>,
    seq: u32,
    notifications: Vec<Value>,
}
impl StdioWallet {
    async fn start(app: &TestApp, wallet: Uuid, secret: &str) -> Self {
        Self::start_with_options(&app.base_url, wallet, secret, &[]).await
    }

    async fn start_with_options(origin: &str, wallet: Uuid, secret: &str, options: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_swissknife"))
            .args(["mcp", "--server", origin, "--wallet", &wallet.to_string()])
            .args(options)
            .env("SWISSKNIFE_MCP_WALLET_KEY", secret)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut mcp = Self {
            child,
            input,
            output,
            seq: 0,
            notifications: Vec::new(),
        };
        let init=mcp.request("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"integration","version":"1"}})).await;
        assert!(init["capabilities"]["tools"].is_object());
        assert_eq!(init["capabilities"]["resources"]["subscribe"], true);
        mcp.write(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await;
        mcp
    }
    async fn write(&mut self, message: Value) {
        self.input.write_all(format!("{message}\n").as_bytes()).await.unwrap();
        self.input.flush().await.unwrap();
    }
    async fn exchange(&mut self, method: &str, params: Value) -> Value {
        self.seq += 1;
        self.write(json!({"jsonrpc":"2.0","id":self.seq,"method":method,"params":params}))
            .await;
        let id = self.seq;
        timeout(Duration::from_secs(35), async {
            loop {
                let line = self.output.next_line().await.unwrap().expect("MCP response before EOF");
                let message: Value = serde_json::from_str(&line).expect("stdout contains only JSON-RPC");
                if message["id"] == id {
                    return message;
                }
                if message.get("method").is_some() {
                    self.notifications.push(message);
                }
            }
        })
        .await
        .expect("MCP response timeout")
    }
    async fn request(&mut self, method: &str, params: Value) -> Value {
        let message = self.exchange(method, params).await;
        assert!(message.get("error").is_none(), "{message}");
        message["result"].clone()
    }

    async fn next_update(&mut self, uri: &str) -> Value {
        timeout(Duration::from_secs(15), async {
            loop {
                if let Some(index) = self
                    .notifications
                    .iter()
                    .position(|m| m["method"] == "notifications/resources/updated" && m["params"]["uri"] == uri)
                {
                    return self.notifications.remove(index);
                }
                let line = self.output.next_line().await.unwrap().expect("MCP update");
                self.notifications.push(serde_json::from_str(&line).unwrap());
            }
        })
        .await
        .expect("resource update")
    }
    async fn tool(&mut self, name: &str, args: Value) -> Value {
        self.request("tools/call", json!({"name":name,"arguments":args})).await
    }
    async fn close(mut self) {
        drop(self.input);
        let exit = timeout(Duration::from_secs(8), self.child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(exit.success());
    }
}

#[tokio::test]
async fn stdio_discovers_tools_and_resources_and_settles_an_approved_payment_once() {
    let app = TestApp::isolated("mcp-wallet", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "mcp-owner").await;
    app.fund_onchain(admin, owner.wallet.id, 30_000).await;
    let api = app.api();
    let keys = format!("/v1/me/wallets/{}/agent-keys", owner.wallet.id);
    let issued=api.post(&keys,Auth::ApiKey(&owner.key),json!({"name":"stdio","capabilities":{"read":true,"receive":true,"spend":true},"expires_at":Utc::now()+ChronoDuration::hours(1)})).await.parse::<IssuedAgentKey>();
    let mut mcp = StdioWallet::start(&app, owner.wallet.id, &issued.key).await;
    mcp.request("resources/subscribe", json!({"uri":"wallet://payments/recent"}))
        .await;
    let contact = json!({"ln_address":"billing@EXAMPLE.COM","label":"Supplier"});
    assert_eq!(mcp.tool("add_contact", contact.clone()).await["isError"], true);
    let editor: IssuedAgentKey = api.post(&keys,Auth::ApiKey(&owner.key),
        json!({"name":"address editor","capabilities":{"manage_contacts":true},"expires_at":Utc::now()+ChronoDuration::hours(1)})).await.parse();
    let mut book = StdioWallet::start(&app, owner.wallet.id, &editor.key).await;
    let saved = book.tool("add_contact", contact.clone()).await;
    assert_eq!(saved["structuredContent"]["ln_address"], "billing@example.com");
    let replay = book.tool("add_contact", contact).await;
    assert_eq!(saved["structuredContent"], replay["structuredContent"]);
    assert_eq!(book.tool("list_saved_contacts", json!({})).await["isError"], true);
    let list = mcp.tool("list_saved_contacts", json!({})).await;
    assert_eq!(
        list["structuredContent"]["contacts"],
        json!([saved["structuredContent"]])
    );
    let history = mcp.tool("list_contacts", json!({})).await;
    assert_eq!(history["structuredContent"]["contacts"], json!([]));
    book.close().await;
    let listing = mcp.request("tools/list", json!({})).await;
    let names = listing["tools"].as_array().unwrap();
    for name in [
        "get_balance",
        "create_invoice",
        "get_invoice",
        "decode_invoice",
        "send_payment",
        "get_payment",
        "list_contacts",
        "add_contact",
        "list_saved_contacts",
        "get_payment_submission",
        "get_lightning_address",
        "send_to_address",
        "lnurl_pay",
    ] {
        assert!(names.iter().any(|tool| tool["name"] == name));
    }
    assert!(!names
        .iter()
        .any(|tool| tool["name"] == "lnurl_withdraw" || tool["name"] == "inspect_lnurl_withdraw"));
    let resources = mcp.request("resources/list", json!({})).await;
    assert_eq!(resources["resources"].as_array().unwrap().len(), 2);
    let balance = mcp.request("resources/read", json!({"uri":"wallet://balance"})).await;
    let before: Value = serde_json::from_str(balance["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(before["available_msat"], "30000000");
    let info = mcp.tool("get_wallet_info", json!({})).await;
    assert_eq!(info["structuredContent"]["id"], owner.wallet.id.to_string());
    let created = mcp
        .tool(
            "create_invoice",
            json!({"amount_msat":"72501","description":"receivable"}),
        )
        .await;
    let decoded = mcp
        .tool(
            "decode_invoice",
            json!({"invoice":created["structuredContent"]["ln_invoice"]["bolt11"]}),
        )
        .await;
    assert_eq!(decoded["structuredContent"]["amount_msat"], "72501");
    let invoice = Counterparty::for_provider(&app.provider).invoice(45001, &unique("mcp-pay"));
    let signed = invoice.parse::<lightning_invoice::Bolt11Invoice>().unwrap();
    use bitcoin::hashes::Hash;
    let bakery = swissknife_l402::Bakery::new(&[9; 32]);
    let macaroon = bakery
        .mint("merchant/report", signed.payment_hash().to_byte_array(), u64::MAX)
        .unwrap();
    let challenge = format!("L402 macaroon=\"{macaroon}\", invoice=\"{invoice}\"");
    let inspected = mcp.tool("decode_l402_challenge", json!({"challenge":challenge})).await;
    assert_eq!(inspected["structuredContent"]["amount_msat"], "45001");
    assert_eq!(inspected["structuredContent"]["invoice"], invoice);
    assert_eq!(
        mcp.tool("decode_l402_challenge", json!({"challenge":"broken"})).await["isError"],
        true
    );
    let mut approval =
        json!({"destination":invoice,"amount_msat":"45000","maximum_fee_msat":"1000","idempotency_key":Uuid::new_v4()});
    assert_eq!(mcp.tool("send_payment", approval.clone()).await["isError"], true);
    approval["amount_msat"] = json!("45001");
    let accepted = mcp.tool("send_payment", approval.clone()).await;
    let id = accepted["structuredContent"]["payment"]["id"]
        .as_str()
        .expect("queued payment UUID")
        .to_owned();
    let replay = mcp.tool("send_payment", approval).await;
    assert_eq!(replay["structuredContent"]["payment"]["id"], id);
    timeout(Duration::from_secs(35), async {
        loop {
            let state = mcp.tool("get_payment_submission", json!({"id":id})).await;
            if state["structuredContent"]["state"] == "settled" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("MCP payment settles");
    let credential = mcp
        .tool(
            "create_l402_authorization",
            json!({"challenge":challenge,"payment_id":id}),
        )
        .await;
    let header = credential["structuredContent"]["authorization"]
        .as_str()
        .expect("L402 credential for settled payment");
    assert_eq!(
        bakery.verify("merchant/report", header, 1).unwrap().payment_hash,
        signed.payment_hash().to_byte_array()
    );
    let different = format!(
        "L402 macaroon=\"{macaroon}\", invoice=\"{}\"",
        Counterparty::for_provider(&app.provider).invoice(45001, &unique("unrelated-l402"))
    );
    let mismatched = mcp
        .tool(
            "create_l402_authorization",
            json!({"challenge":different,"payment_id":id}),
        )
        .await;
    assert_eq!(mismatched["isError"], true);
    assert!(mismatched["structuredContent"].get("authorization").is_none());
    mcp.next_update("wallet://payments/recent").await;
    let history = mcp
        .request("resources/read", json!({"uri":"wallet://payments/recent"}))
        .await;
    let history: Value = serde_json::from_str(history["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(history["payments"].as_array().unwrap().len(), 1);
    assert_eq!(history["payments"][0]["amount_msat"], "45001");
    check_payment_history(&app, &mut mcp, admin, owner.wallet.id, &history["payments"]).await;
    assert_status(
        &api.delete(&format!("{keys}/{}", issued.metadata.id), Auth::ApiKey(&owner.key))
            .await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(mcp.tool("get_balance", json!({})).await["isError"], true);
    let denied = mcp
        .exchange("resources/subscribe", json!({"uri":"wallet://balance"}))
        .await;
    assert!(denied["error"].is_object());
    mcp.close().await;
}

async fn check_payment_history(
    app: &TestApp,
    mcp: &mut StdioWallet,
    admin: &str,
    wallet_id: Uuid,
    lightning_history: &Value,
) {
    use sea_orm::{ConnectionTrait, Database};

    let api = app.api();
    // Record an internal payment so filtering happens against a mixed wallet history.
    let payee = app.create_wallet(admin, "mcp-history-payee").await;
    let invoice = api
        .post(
            "/v1/invoices",
            Auth::Bearer(admin),
            json!({"wallet_id":payee.id,"amount_msat":46002}),
        )
        .await;
    assert_status(&invoice, StatusCode::OK);
    let internal = api
        .post(
            "/v1/payments",
            Auth::Bearer(admin),
            json!({"wallet_id":wallet_id,"input":invoice.body["ln_invoice"]["bolt11"]}),
        )
        .await;
    assert_status(&internal, StatusCode::OK);
    let filtered = mcp
        .tool(
            "list_payments",
            json!({"status":"Settled","ledger":"Lightning","limit":1}),
        )
        .await;
    assert_eq!(filtered["structuredContent"]["payments"], *lightning_history);
    for (query, count) in [
        (json!({"ledger":"Internal"}), 1),
        (json!({"status":"Failed"}), 0),
        (json!({"status":"Settled","ledger":"Lightning","offset":1}), 0),
        (json!({}), 2),
    ] {
        let listed = mcp.tool("list_payments", query).await;
        assert_eq!(listed["structuredContent"]["payments"].as_array().unwrap().len(), count);
    }
    assert_eq!(
        mcp.tool("list_payments", json!({"ledger":"Card"})).await["isError"],
        true
    );

    // Simulate batched historical records in this test's isolated database.
    let db = Database::connect(&app.database_url).await.unwrap();
    db.execute_unprepared("UPDATE payment SET created_at = (SELECT MIN(created_at) FROM payment)")
        .await
        .unwrap();
    db.close().await.unwrap();
    let mut ids = [
        lightning_history[0]["id"].as_str().unwrap(),
        internal.body["id"].as_str().unwrap(),
    ];
    ids.sort_unstable_by(|a, b| b.cmp(a));
    for (offset, expected) in ids.into_iter().enumerate() {
        let page = mcp
            .tool("list_payments", json!({"status":"Settled","offset":offset,"limit":1}))
            .await;
        assert_eq!(page["structuredContent"]["payments"][0]["id"], expected);
    }
}

#[tokio::test]
async fn stdio_invoice_receipt_confirms_payment_only_for_the_authorized_wallet() {
    let app = TestApp::isolated("mcp-invoice-receipt", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "mcp-invoice-owner").await;
    let other = app.create_account_with_wallet(admin, "mcp-invoice-other").await;
    let api = app.api();

    async fn connect(
        app: &TestApp,
        owner: &crate::common::fixtures::TestAccount,
        name: &str,
        capabilities: Value,
    ) -> (StdioWallet, IssuedAgentKey) {
        let issued: IssuedAgentKey = app
            .api()
            .post(
                &format!("/v1/me/wallets/{}/agent-keys", owner.wallet.id),
                Auth::ApiKey(&owner.key),
                json!({"name":name, "capabilities":capabilities, "expires_at":Utc::now()+ChronoDuration::hours(1)}),
            )
            .await
            .parse();
        (StdioWallet::start(app, owner.wallet.id, &issued.key).await, issued)
    }

    let (mut issuer, _) = connect(&app, &owner, "issuer", json!({"receive":true})).await;
    let (mut reader, read_key) = connect(&app, &owner, "reader", json!({"read":true})).await;
    let (mut foreign, _) = connect(&app, &other, "foreign", json!({"read":true})).await;
    let created = issuer
        .tool(
            "create_invoice",
            json!({"amount_msat":"101001","description":"MCP receipt"}),
        )
        .await;
    let invoice = &created["structuredContent"];
    let lookup = json!({"id":invoice["id"].as_str().expect("issued invoice id")});
    assert_eq!(issuer.tool("get_invoice", lookup.clone()).await["isError"], true);
    assert_eq!(foreign.tool("get_invoice", lookup.clone()).await["isError"], true);
    assert_eq!(
        reader.tool("get_invoice", json!({"id":"invalid"})).await["isError"],
        true
    );
    let pending = reader.tool("get_invoice", lookup.clone()).await;
    assert_eq!(pending["structuredContent"]["status"], "Pending");
    assert_eq!(pending["structuredContent"]["amount_msat"], "101001");

    Counterparty::for_provider(&app.provider).pay(invoice["ln_invoice"]["bolt11"].as_str().unwrap());
    let settled = timeout(Duration::from_secs(40), async {
        loop {
            let answer = reader.tool("get_invoice", lookup.clone()).await;
            assert_ne!(answer["isError"], true, "{answer}");
            if answer["structuredContent"]["status"] == "Settled" {
                break answer["structuredContent"].clone();
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("the MCP receipt must observe provider settlement");
    assert_eq!(settled["id"], invoice["id"]);
    assert_eq!(settled["amount_received_msat"], "101001");
    assert!(settled["payment_time"].is_string());
    assert!(settled.get("ln_invoice").is_none());
    assert_eq!(
        reader.tool("get_invoice", lookup.clone()).await["structuredContent"],
        settled
    );
    let path = format!("/v1/me/wallets/{}/agent-keys/{}", owner.wallet.id, read_key.metadata.id);
    assert_status(
        &api.delete(&path, Auth::ApiKey(&owner.key)).await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(reader.tool("get_invoice", lookup).await["isError"], true);
    issuer.close().await;
    reader.close().await;
    foreign.close().await;
}

#[tokio::test]
async fn stdio_balance_subscription_recovers_from_an_outage_and_stops_after_revocation() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::Notify;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, Request, ResponseTemplate,
    };

    let api = MockServer::start().await;
    let wallet = Uuid::new_v4();
    let route = format!("/v1/me/wallets/{wallet}/balance");
    let calls = AtomicUsize::new(0);
    Mock::given(method("GET"))
        .and(path(route.clone()))
        .respond_with(move |_: &Request| match calls.fetch_add(1, Ordering::Relaxed) {
            0 => ResponseTemplate::new(200).set_body_json(json!({"available_msat":100})),
            1 => ResponseTemplate::new(503),
            _ => ResponseTemplate::new(200).set_body_json(json!({"available_msat":200})),
        })
        .mount(&api)
        .await;

    let key = format!("swka_{}", hex::encode([31; 32]));
    let mut mcp = StdioWallet::start_with_options(&api.uri(), wallet, &key, &[]).await;
    let uri = "wallet://balance";
    mcp.request("resources/subscribe", json!({"uri":uri})).await;
    mcp.next_update(uri).await;
    let read = mcp.request("resources/read", json!({"uri":uri})).await;
    let snapshot: Value = serde_json::from_str(read["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(snapshot["available_msat"], "200");

    let denied = Arc::new(Notify::new());
    let revocation = denied.clone();
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(move |_: &Request| {
            revocation.notify_one();
            ResponseTemplate::new(401)
        })
        .with_priority(1)
        .expect(1)
        .mount(&api)
        .await;
    timeout(Duration::from_secs(8), denied.notified())
        .await
        .expect("poll after recovery");
    // A retry after the first failed poll would happen within six seconds.
    assert!(timeout(Duration::from_secs(8), denied.notified()).await.is_err());
    mcp.close().await;
}

#[tokio::test]
async fn stdio_resource_subscription_tracks_only_the_selected_wallet_until_unsubscribe() {
    let app = TestApp::isolated("mcp-subscriptions", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "subscribed-wallet").await;
    let keys = format!("/v1/me/wallets/{}/agent-keys", owner.wallet.id);
    let issued: IssuedAgentKey = app.api().post(&keys, Auth::ApiKey(&owner.key),
        json!({"name":"subscriptions","capabilities":{"read":true},"expires_at":Utc::now()+ChronoDuration::hours(1)})).await.parse();
    let mut mcp = StdioWallet::start(&app, owner.wallet.id, &issued.key).await;
    let balance = json!({"uri":"wallet://balance"});
    mcp.request("resources/subscribe", balance.clone()).await;
    mcp.request("resources/subscribe", balance.clone()).await;
    let other = app.create_account_with_wallet(admin, "different-wallet").await;
    app.fund_onchain(admin, other.wallet.id, 10001).await;
    assert!(timeout(Duration::from_secs(4), mcp.output.next_line()).await.is_err());
    app.fund_onchain(admin, owner.wallet.id, 25001).await;
    let message = mcp.next_update("wallet://balance").await;
    assert!(message["params"].get("contents").is_none());
    let read = mcp.request("resources/read", balance.clone()).await;
    let snapshot: Value = serde_json::from_str(read["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(snapshot["available_msat"], "25001000");
    mcp.request("resources/unsubscribe", balance).await;
    app.fund_onchain(admin, owner.wallet.id, 20001).await;
    assert!(timeout(Duration::from_secs(4), mcp.output.next_line()).await.is_err());
    mcp.close().await;
}

#[tokio::test]
async fn streamable_http_uses_a_separate_transport_credential_and_enforces_wallet_permissions() {
    let app = TestApp::isolated("mcp-http", &[]).await;
    let owner = app
        .create_account_with_wallet(app.admin_token().await, "mcp-http-owner")
        .await;
    let api = app.api();
    let keys = format!("/v1/me/wallets/{}/agent-keys", owner.wallet.id);
    let issued = api
        .post(
            &keys,
            Auth::ApiKey(&owner.key),
            json!({"name":"remote read","capabilities":{"read":true},
        "expires_at":Utc::now()+ChronoDuration::hours(1)}),
        )
        .await
        .parse::<IssuedAgentKey>();
    let token = hex::encode([23; 32]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_swissknife"))
        .args([
            "mcp",
            "--server",
            &app.base_url,
            "--wallet",
            &owner.wallet.id.to_string(),
            "--http-listen",
            "127.0.0.1:0",
        ])
        .env("SWISSKNIFE_MCP_WALLET_KEY", &issued.key)
        .env("SWISSKNIFE_MCP_HTTP_TOKEN", &token)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut log = BufReader::new(child.stderr.take().unwrap()).lines();
    let line = timeout(Duration::from_secs(10), log.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let url = line.strip_prefix("Serving wallet MCP on ").unwrap();
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let call = |method: &str, params: Value| {
        http.post(url)
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-11-25")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
    };
    for bad in ["", "wrong", issued.key.as_str()] {
        assert_eq!(
            call("tools/list", json!({}))
                .bearer_auth(bad)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        call("tools/list", json!({}))
            .bearer_auth(&token)
            .header("Origin", "https://unrelated.example")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call("tools/list", json!({}))
            .bearer_auth(&token)
            .header("Host", "unrelated.example")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    async fn rpc(request: reqwest::RequestBuilder, token: &str) -> Value {
        let response = request.bearer_auth(token).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let content_type = response.headers()["content-type"].to_str().unwrap().to_owned();
        let text = response.text().await.unwrap();
        let message: Value = if content_type.starts_with("text/event-stream") {
            text.lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok())
                .find(|value| value["id"] == 1)
                .unwrap()
        } else {
            serde_json::from_str(&text).unwrap()
        };
        assert!(message.get("error").is_none(), "{message}");
        message["result"].clone()
    }
    let init = rpc(
        call(
            "initialize",
            json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"remote-test","version":"1"}}),
        ),
        &token,
    )
    .await;
    assert!(init["capabilities"]["tools"].is_object());
    assert!(init["capabilities"]["resources"].get("subscribe").is_none());
    let balance = rpc(call("tools/call", json!({"name":"get_balance","arguments":{}})), &token).await;
    assert_eq!(balance["structuredContent"]["available_msat"], "0");
    let invoice = rpc(
        call(
            "tools/call",
            json!({"name":"create_invoice","arguments":{"amount_msat":"5000","description":"unpermitted"}}),
        ),
        &token,
    )
    .await;
    assert_eq!(invoice["isError"], true);
    let mut stream = http.post(url)
        .bearer_auth(&token)
        .timeout(Duration::from_secs(45))
        .header("Accept","application/json, text/event-stream")
        .header("MCP-Protocol-Version","2026-07-28")
        .header("Mcp-Method","subscriptions/listen")
        .json(&json!({"jsonrpc":"2.0","id":9,"method":"subscriptions/listen","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}},"notifications":{"resourceSubscriptions":["wallet://balance","wallet://unknown"]}}}))
        .send().await.unwrap();
    if stream.status() != StatusCode::OK {
        panic!(
            "Subscription failed with {}: {}",
            stream.status(),
            stream.text().await.unwrap()
        );
    }
    let mut events = Vec::new();
    async fn wait_for(stream: &mut reqwest::Response, events: &mut Vec<u8>, count: usize) {
        timeout(Duration::from_secs(15), async {
            loop {
                let data = String::from_utf8_lossy(events);
                let seen = data
                    .lines()
                    .filter_map(|l| l.strip_prefix("data:"))
                    .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
                    .filter(|x| {
                        x["method"] == "notifications/resources/updated" && x["params"]["uri"] == "wallet://balance"
                    })
                    .count();
                if seen >= count {
                    return;
                }
                events.extend(stream.chunk().await.unwrap().expect("active subscription"));
            }
        })
        .await
        .expect("wallet update");
    }
    wait_for(&mut stream, &mut events, 1).await;
    let initial = String::from_utf8_lossy(&events);
    assert!(initial.contains("notifications/subscriptions/acknowledged"));
    assert!(!initial.contains("wallet://unknown"));
    app.fund_onchain(app.admin_token().await, owner.wallet.id, 16001).await;
    wait_for(&mut stream, &mut events, 2).await;
    assert_status(
        &api.delete(&format!("{keys}/{}", issued.metadata.id), Auth::ApiKey(&owner.key))
            .await,
        StatusCode::NO_CONTENT,
    );
    let rest = timeout(Duration::from_secs(8), stream.text()).await.unwrap().unwrap();
    assert!(!rest.contains("notifications/resources/updated"));
    let revoked = rpc(call("tools/call", json!({"name":"get_balance","arguments":{}})), &token).await;
    assert_eq!(revoked["isError"], true);
    child.kill().await.unwrap();
    app.stop();
}

#[tokio::test]
async fn withdraw_voucher_uses_the_selected_invoice_and_reconciles_callback_outcomes() {
    use lnurl::lnurl::LnUrl;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    let app = TestApp::isolated("mcp-withdraw-voucher", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "voucher-owner").await;
    let issued: IssuedAgentKey = app.api().post(
        &format!("/v1/me/wallets/{}/agent-keys", owner.wallet.id), Auth::ApiKey(&owner.key),
        json!({"name":"receive vouchers","capabilities":{"receive":true,"read":true},"expires_at":Utc::now()+ChronoDuration::hours(1)}),
    ).await.parse();
    let service = MockServer::start().await;
    let origin = service.uri();
    let voucher = |name: &str| {
        LnUrl {
            url: format!("{origin}/{name}"),
        }
        .encode()
    };
    for (name, callback, min, max) in [
        ("valid", format!("{origin}/redeem?order=42&k1=from-discovery"), 1, 90000),
        ("rejected", format!("{origin}/decline"), 1, 90000),
        ("uncertain", format!("{origin}/lost-response"), 1, 90000),
        ("empty", format!("{origin}/redeem"), 0, 0),
        (
            "foreign-callback",
            "https://unconfigured.example/redeem".to_string(),
            1,
            90000,
        ),
    ] {
        Mock::given(method("GET")).and(path(format!("/{name}"))).respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"tag":"withdrawRequest","callback":callback,"k1":"fixture-voucher-secret","defaultDescription":"payout","minWithdrawable":min,"maxWithdrawable":max})
        )).mount(&service).await;
    }
    Mock::given(method("GET"))
        .and(path("/redeem"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"OK"})))
        .expect(1)
        .mount(&service)
        .await;
    Mock::given(method("GET"))
        .and(path("/decline"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"ERROR","reason":"declined"})))
        .expect(1)
        .mount(&service)
        .await;
    Mock::given(method("GET"))
        .and(path("/lost-response"))
        .respond_with(ResponseTemplate::new(502))
        .expect(1)
        .mount(&service)
        .await;

    let mut mcp = StdioWallet::start_with_options(
        &app.base_url,
        owner.wallet.id,
        &issued.key,
        &["--withdraw-origin", &origin],
    )
    .await;
    let names = mcp.request("tools/list", json!({})).await;
    for name in ["lnurl_withdraw", "inspect_lnurl_withdraw"] {
        assert!(names["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == name));
    }
    let info = mcp
        .tool(
            "inspect_lnurl_withdraw",
            json!({"lnurl":format!("LIGHTNING:{}",voucher("valid").to_uppercase())}),
        )
        .await;
    assert_eq!(
        info["structuredContent"],
        json!({"issuer":origin,"callback_origin":origin,"description":"payout","min_msat":"1","max_msat":"90000"})
    );
    let invalid = mcp
        .tool("inspect_lnurl_withdraw", json!({"lnurl":voucher("foreign-callback")}))
        .await;
    assert_eq!(invalid["isError"], true);
    let empty = mcp
        .tool("inspect_lnurl_withdraw", json!({"lnurl":voucher("empty")}))
        .await;
    assert_eq!(empty["structuredContent"]["max_msat"], "0");

    let created = mcp
        .tool("create_invoice", json!({"amount_msat":"72001","description":"payout"}))
        .await;
    let id = created["structuredContent"]["id"].as_str().unwrap();
    let bolt11 = created["structuredContent"]["ln_invoice"]["bolt11"].as_str().unwrap();
    let claim = |v: &str, amount: &str| json!({"lnurl":voucher(v),"invoice_id":id,"amount_msat":amount});
    let other = app.create_wallet(admin, "other-voucher-wallet").await;
    let foreign = app
        .api()
        .post(
            "/v1/invoices",
            Auth::Bearer(admin),
            json!({"wallet_id":other.id,"amount_msat":72001,"description":"another wallet's invoice"}),
        )
        .await
        .parse::<swissknife_types::Invoice>();
    let mut foreign_claim = claim("valid", "72001");
    foreign_claim["invoice_id"] = json!(foreign.id);
    assert_eq!(mcp.tool("lnurl_withdraw", foreign_claim).await["isError"], true);

    assert_eq!(
        mcp.tool("lnurl_withdraw", claim("valid", "72000")).await["isError"],
        true
    );
    assert_eq!(
        mcp.tool("lnurl_withdraw", claim("empty", "72001")).await["isError"],
        true
    );
    for (link, outcome) in [
        ("rejected", "rejected"),
        ("uncertain", "unknown"),
        ("valid", "accepted"),
    ] {
        let result = mcp.tool("lnurl_withdraw", claim(link, "72001")).await;
        assert_eq!(
            result["structuredContent"],
            json!({"invoice_id":id,"amount_msat":"72001","callback_status":outcome})
        );
    }
    let requests = service.received_requests().await.unwrap();
    assert!(requests
        .iter()
        .all(|r| !r.headers.contains_key("wallet-key") && !r.headers.contains_key("authorization")));
    let accepted = requests.iter().find(|r| r.url.path() == "/redeem").unwrap();
    let params: std::collections::BTreeMap<_, _> = accepted.url.query_pairs().collect();
    assert_eq!(
        accepted
            .url
            .query_pairs()
            .filter(|(key, _)| key == "k1" || key == "pr")
            .count(),
        2
    );
    assert_eq!(params.get("k1").unwrap(), "fixture-voucher-secret");
    assert_eq!(params.get("order").unwrap(), "42");
    assert_eq!(params.get("pr").unwrap(), bolt11);
    let lookup = json!({"id":id});
    assert_eq!(
        mcp.tool("get_invoice", lookup.clone()).await["structuredContent"]["status"],
        "Pending"
    );
    Counterparty::for_provider(&app.provider).pay(bolt11);
    timeout(Duration::from_secs(40), async {
        loop {
            let receipt = mcp.tool("get_invoice", lookup.clone()).await;
            if receipt["structuredContent"]["status"] == "Settled" {
                assert_eq!(receipt["structuredContent"]["amount_received_msat"], "72001");
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("issuer paid the selected invoice");
    assert_eq!(
        mcp.tool("lnurl_withdraw", claim("valid", "72001")).await["isError"],
        true
    );
    mcp.close().await;
}

#[tokio::test]
async fn cached_l402_pass_inspection_keeps_payment_credentials_local() {
    use bitcoin::hashes::{sha256, Hash};
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    let app = TestApp::isolated("mcp-pass", &[]).await;
    let merchant = MockServer::start().await;
    let other = MockServer::start().await;
    let report = format!("{}/l402/report", merchant.uri());
    let redirect = format!("{}/l402/moved", merchant.uri());
    let unsupported = format!("{}/l402/other", merchant.uri());
    let unreachable = format!("{}/l402/unlisted", other.uri());
    Mock::given(method("HEAD"))
        .and(path("/l402/report/status"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("SwissKnife-L402-Status", "active")
                .insert_header("SwissKnife-L402-Remaining", "1"),
        )
        .expect(2)
        .mount(&merchant)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/l402/moved/status"))
        .respond_with(ResponseTemplate::new(307).insert_header("Location", unreachable.as_str()))
        .expect(1)
        .mount(&merchant)
        .await;

    let secret = format!("swka_{}", hex::encode([31; 32]));
    let mut mcp = StdioWallet::start_with_options(
        &app.base_url,
        Uuid::new_v4(),
        &secret,
        &[
            "--l402-resource",
            &report,
            "--l402-resource",
            &redirect,
            "--l402-resource",
            &unsupported,
        ],
    )
    .await;
    let bakery = swissknife_l402::Bakery::new(&[44; 32]);
    let preimage = [13; 32];
    let macaroon = bakery
        .mint("report", sha256::Hash::hash(&preimage).to_byte_array(), u64::MAX)
        .unwrap();
    let preimage = hex::encode(preimage);
    let token = format!("L402 {macaroon}:{preimage}");
    let args = |url: &str, proof: &str| json!({"resource_url":url, "authorization":proof});
    for url in [&unreachable, &format!("{report}?other=1"), &format!("{report}/other")] {
        assert_eq!(
            mcp.tool("check_l402_token", args(url, &token)).await["structuredContent"],
            json!({"status":"unknown"})
        );
    }
    let invalid = format!("L402 {macaroon}:{}", hex::encode([14; 32]));
    assert_eq!(
        mcp.tool("check_l402_token", args(&report, &invalid)).await["structuredContent"],
        json!({"status":"invalid"})
    );
    assert!(merchant.received_requests().await.unwrap().is_empty());

    for _ in 0..2 {
        assert_eq!(
            mcp.tool("check_l402_token", args(&report, &token)).await["structuredContent"],
            json!({"status":"active","remaining_requests":1})
        );
    }
    for url in [&redirect, &unsupported] {
        assert_eq!(
            mcp.tool("check_l402_token", args(url, &token)).await["structuredContent"],
            json!({"status":"unknown"})
        );
    }
    assert!(other.received_requests().await.unwrap().is_empty());
    let requests = merchant.received_requests().await.unwrap();
    assert_eq!(requests.len(), 4);
    for request in requests {
        assert_eq!(request.method.as_str(), "HEAD");
        assert!(request.body.is_empty());
        assert!(!request.headers.contains_key("authorization"));
        assert!(!request.headers.contains_key("wallet-key"));
        let inspection = request.headers["swissknife-l402-inspection"].to_str().unwrap();
        assert_ne!(inspection, macaroon);
        assert!(bakery
            .verify("report", &format!("L402 {inspection}:{preimage}"), 1)
            .is_err());
        for value in request.headers.values() {
            let value = value.to_str().unwrap();
            assert!(!value.contains(&preimage) && !value.contains(&macaroon) && !value.contains(&secret));
        }
    }
    mcp.close().await;
}
