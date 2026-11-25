//! Verify one approved withdrawal against the independent Bitcoin Core mempool and wallet accounting.
use crate::common::{
    assert_error, assert_status,
    chain::{bitcoin_rpc, mine, new_address},
    wait::wait_until,
    Auth, TestApp,
};
use reqwest::StatusCode;
use serde_json::json;
use std::time::Duration;
use swissknife_types::{PaymentExecutionState as State, PaymentSubmission};
use uuid::Uuid;

#[tokio::test]
async fn withdrawal_reserves_approved_cap_broadcasts_and_settles_once_after_restart() {
    let app = TestApp::isolated("withdrawal-queue", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "withdrawal-owner").await;
    let wallet = owner.wallet.id;
    app.fund_onchain(admin, wallet, 100_000).await;
    let before = app.wallet_balance(admin, wallet).await.available_msat;
    let path = format!("/v1/me/wallets/{wallet}/payments/submissions");
    let api = app.api();
    let auth = Auth::ApiKey(&owner.key);
    let dest = new_address().await;
    let mut request = json!({"input":dest,"amount_msat":15_000_000,"idempotency_key":Uuid::new_v4()});
    assert_error(&api.post(&path, auth, &request).await, StatusCode::UNPROCESSABLE_ENTITY);
    request["maximum_fee_msat"] = json!(5_000_000);
    let accepted = api.post(&path, auth, &request).await;
    assert_status(&accepted, StatusCode::ACCEPTED);
    let accepted: PaymentSubmission = accepted.parse();
    assert_eq!(accepted.state, State::Accepted);
    assert!(accepted.payment.bitcoin.is_none());
    let details = accepted.withdrawal.as_ref().unwrap();
    assert_eq!(details.address, dest);
    assert_eq!(details.maximum_fee_msat, 5_000_000);
    let url = &accepted.status_url;
    wait_until(Duration::from_secs(50), "withdrawal in mempool", || async {
        api.get(url, auth).await.parse::<PaymentSubmission>().state == State::Broadcast
    })
    .await;
    let receipt = api.get(url, auth).await.parse::<PaymentSubmission>();
    let txid = receipt.payment.bitcoin.as_ref().unwrap().txid.clone();
    let raw = bitcoin_rpc("getrawtransaction", json!([txid, true])).await;
    assert_eq!(raw["txid"], txid);
    assert!(raw["vout"]
        .as_array()
        .unwrap()
        .iter()
        .any(|o| o["scriptPubKey"]["address"] == dest && o["value"].as_f64() == Some(0.00015)));
    let entry = bitcoin_rpc("getmempoolentry", json!([txid])).await;
    let miner_fee_msat = (entry["fees"]["base"].as_f64().unwrap() * 100_000_000.0).round() as u64 * 1000;
    assert_eq!(receipt.payment.fee_msat, Some(miner_fee_msat));
    let held = app.wallet_balance(admin, wallet).await;
    assert_eq!(held.reserved_msat, 20_000_000);
    assert_eq!(held.available_msat, before - held.reserved_msat as i64);
    let duplicate: PaymentSubmission = api.post(&path, auth, &request).await.parse();
    assert_eq!(duplicate.payment.id, accepted.payment.id);
    request["maximum_fee_msat"] = json!(5_001_000);
    assert_error(&api.post(&path, auth, &request).await, StatusCode::CONFLICT);
    assert_error(&api.delete(url, auth).await, StatusCode::CONFLICT);
    app.stop();
    mine(3).await;
    let restarted = app.restart("withdrawal-committed", &[]).await;
    wait_until(
        Duration::from_secs(90),
        "withdrawal confirmed after restart",
        || async { restarted.api().get(url, auth).await.parse::<PaymentSubmission>().state == State::Settled },
    )
    .await;
    let settled: PaymentSubmission = restarted.api().get(url, auth).await.parse();
    assert_eq!(settled.payment.bitcoin.unwrap().txid, txid);
    let fee = settled.payment.fee_msat.unwrap();
    assert!(fee > 0 && fee <= 5_000_000);
    let balance = restarted.wallet_balance(admin, wallet).await;
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(balance.available_msat, before - 15_000_000 - fee as i64);

    request["idempotency_key"] = json!(Uuid::new_v4());
    request["maximum_fee_msat"] = json!(0);
    let failed: PaymentSubmission = restarted.api().post(&path, auth, &request).await.parse();
    wait_until(
        Duration::from_secs(50),
        "fee cap enforced before publication",
        || async {
            restarted
                .api()
                .get(&failed.status_url, auth)
                .await
                .parse::<PaymentSubmission>()
                .state
                == State::Failed
        },
    )
    .await;
    let unchanged = restarted.wallet_balance(admin, wallet).await;
    assert_eq!(
        (unchanged.available_msat, unchanged.reserved_msat),
        (balance.available_msat, 0)
    );
}

#[tokio::test]
async fn registered_bitcoin_address_settles_idempotently_without_a_miner_fee() {
    use swissknife_types::BtcAddress;
    let app = TestApp::isolated("registered-withdrawal-submission", &[]).await;
    let admin = app.admin_token().await;
    let payer = app.create_account_with_wallet(admin, "registered-payer").await;
    let receiver = app.create_account_with_wallet(admin, "registered-receiver").await;
    app.fund_onchain(admin, payer.wallet.id, 20_000).await;
    let api = app.api();
    let receiver_url = format!("/v1/me/wallets/{}/bitcoin/addresses", receiver.wallet.id);
    let address: BtcAddress = api
        .post(&receiver_url, Auth::ApiKey(&receiver.key), json!({}))
        .await
        .parse();
    let path = format!("/v1/me/wallets/{}/payments/submissions", payer.wallet.id);
    let body = json!({"input":address.address,"amount_msat":5_000_000,"idempotency_key":Uuid::new_v4()});
    let (a, b) = tokio::join!(
        api.post(&path, Auth::ApiKey(&payer.key), &body),
        api.post(&path, Auth::ApiKey(&payer.key), &body)
    );
    assert_status(&a, StatusCode::ACCEPTED);
    assert_status(&b, StatusCode::ACCEPTED);
    let a: PaymentSubmission = a.parse();
    let b: PaymentSubmission = b.parse();
    assert_eq!(a.payment.id, b.payment.id);
    assert_eq!(a.state, State::Settled);
    assert!(a.withdrawal.is_none());
    assert_eq!(a.payment.fee_msat, Some(0));
    assert_eq!(
        app.wallet_balance(admin, payer.wallet.id).await.available_msat,
        15_000_000
    );
    assert_eq!(
        app.wallet_balance(admin, receiver.wallet.id).await.available_msat,
        5_000_000
    );
}
