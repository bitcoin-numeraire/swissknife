//! Real Lightning flows over the LND<->CLN channel — the `lightning` tier.
//! These reach the LN provider, so they belong on the provider matrix.

use std::time::Duration;

use reqwest::StatusCode;
use serde_json::json;
use tokio::time::{sleep, Instant};

use crate::common::counterparty::Counterparty;
use crate::common::{app, assert_status, Auth, TestApp};

const SETTLE_TIMEOUT: Duration = Duration::from_secs(45);

async fn available_msat(app: &TestApp, token: &str) -> i64 {
    let res = app.api().get("/v1/me/balance", Auth::Bearer(token)).await;
    assert_status(&res, StatusCode::OK);
    res.body["available_msat"].as_i64().unwrap_or_default()
}

mod receive {
    use super::*;

    #[tokio::test]
    async fn a_counterparty_payment_credits_the_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let amount_msat = 250_000_000_i64; // 250k sat, well within channel liquidity

        let before = available_msat(app, token).await;

        // SwissKnife issues a real invoice on the node under test.
        let res = app
            .api()
            .post(
                "/v1/me/invoices",
                Auth::Bearer(token),
                json!({ "amount_msat": amount_msat, "description": "itest receive" }),
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let bolt11 = res.body["ln_invoice"]["bolt11"]
            .as_str()
            .expect("invoice has a bolt11")
            .to_string();

        // The counterparty pays it over the channel.
        Counterparty::for_provider(&app.provider).pay(&bolt11);

        // SwissKnife's event listener should detect the settlement and credit the wallet.
        let started = Instant::now();
        loop {
            let now = available_msat(app, token).await;
            if now >= before + amount_msat {
                break;
            }
            assert!(
                started.elapsed() < SETTLE_TIMEOUT,
                "wallet was not credited after receiving a payment (before={before}, now={now})"
            );
            sleep(Duration::from_millis(500)).await;
        }
    }
}
