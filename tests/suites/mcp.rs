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
        };
        let init=mcp.request("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"integration","version":"1"}})).await;
        assert!(init["capabilities"]["tools"].is_object());
        mcp.write(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await;
        mcp
    }
    async fn write(&mut self, message: Value) {
        self.input.write_all(format!("{message}\n").as_bytes()).await.unwrap();
        self.input.flush().await.unwrap();
    }
    async fn request(&mut self, method: &str, params: Value) -> Value {
        self.seq += 1;
        self.write(json!({"jsonrpc":"2.0","id":self.seq,"method":method,"params":params}))
            .await;
        let id = self.seq;
        timeout(Duration::from_secs(35), async {
            loop {
                let line = self.output.next_line().await.unwrap().expect("MCP response before EOF");
                let message: Value = serde_json::from_str(&line).expect("stdout contains only JSON-RPC");
                if message["id"] == id {
                    assert!(message.get("error").is_none(), "{message}");
                    return message["result"].clone();
                }
            }
        })
        .await
        .expect("MCP response timeout")
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
    mcp.close().await;
}
