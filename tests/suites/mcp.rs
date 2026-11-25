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
        let mut child = Command::new(env!("CARGO_BIN_EXE_swissknife"))
            .args(["mcp", "--server", &app.base_url, "--wallet", &wallet.to_string()])
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
        timeout(Duration::from_secs(8), async {
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
    let listing = mcp.request("tools/list", json!({})).await;
    let names = listing["tools"].as_array().unwrap();
    for name in [
        "get_balance",
        "create_invoice",
        "decode_invoice",
        "send_payment",
        "get_payment",
        "list_contacts",
        "get_payment_submission",
        "get_lightning_address",
        "send_to_address",
        "lnurl_pay",
    ] {
        assert!(names.iter().any(|tool| tool["name"] == name));
    }
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
