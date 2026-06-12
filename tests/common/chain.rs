//! Minimal bitcoind regtest control for funding wallets on-chain. Talks to the
//! miner wallet over JSON-RPC; endpoint/credentials are overridable via env.

use serde_json::{json, Value};

fn rpc_url() -> String {
    std::env::var("SWISSKNIFE_ITEST_BITCOIN_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:18443/wallet/miner".to_string())
}

pub async fn bitcoin_rpc(method: &str, params: Value) -> Value {
    let user = std::env::var("SWISSKNIFE_ITEST_BITCOIN_RPC_USER").unwrap_or_else(|_| "regtest".to_string());
    let password = std::env::var("SWISSKNIFE_ITEST_BITCOIN_RPC_PASSWORD").unwrap_or_else(|_| "regtest".to_string());

    let response = reqwest::Client::new()
        .post(rpc_url())
        .basic_auth(user, Some(password))
        .json(&json!({ "jsonrpc": "2.0", "id": "itest", "method": method, "params": params }))
        .send()
        .await
        .unwrap_or_else(|e| panic!("bitcoind rpc {method} request failed: {e}"));

    let body: Value = response.json().await.expect("bitcoind rpc response is json");
    assert!(
        body["error"].is_null(),
        "bitcoind rpc {method} returned error: {}",
        body["error"]
    );
    body["result"].clone()
}

/// Send `sats` to `address` from the miner wallet (does not mine).
pub async fn send_to_address(address: &str, sats: u64) {
    let btc = format!("{:.8}", sats as f64 / 100_000_000.0);
    bitcoin_rpc("sendtoaddress", json!([address, btc])).await;
}

/// Mine `blocks` to the miner wallet, confirming pending transactions.
pub async fn mine(blocks: u64) {
    let address = bitcoin_rpc("getnewaddress", json!(["itest", "bech32"])).await;
    let address = address.as_str().expect("miner address");
    bitcoin_rpc("generatetoaddress", json!([blocks, address])).await;
}
