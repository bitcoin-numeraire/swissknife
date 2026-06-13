//! Real Lightning flows over the LND<->CLN channel. These reach the LN
//! provider, so they run across the provider matrix. Each test uses its own
//! wallet for balance isolation.

use std::time::Duration;

use reqwest::StatusCode;

use swissknife_types::{Invoice, NewInvoiceRequest, Payment, PaymentStatus, SendPaymentRequest};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::unique;
use crate::common::wait::wait_until;
use crate::common::{app, assert_status, Auth};

mod receive {
    use super::*;

    #[tokio::test]
    async fn a_counterparty_payment_credits_the_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "ln-receive").await;
        let amount_msat = 250_000_000u64;

        // SwissKnife issues a real invoice for the wallet.
        let res = app
            .api()
            .post(
                "/v1/invoices",
                Auth::Bearer(token),
                NewInvoiceRequest {
                    wallet_id: Some(wallet.id),
                    amount_msat,
                    description: Some("itest receive".to_string()),
                    expiry: None,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let bolt11 = res.parse::<Invoice>().ln_invoice.expect("invoice has a bolt11").bolt11;

        // The counterparty pays it over the channel.
        Counterparty::for_provider(&app.provider).pay(&bolt11);

        // The event listener should detect settlement and credit the wallet.
        wait_until(Duration::from_secs(45), "wallet credited after receiving", || async {
            app.wallet_balance(token, wallet.id).await.available_msat >= amount_msat as i64
        })
        .await;
    }
}

mod send {
    use super::*;

    #[tokio::test]
    async fn pays_a_counterparty_invoice_and_debits_the_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "ln-send").await;

        app.fund_onchain(token, wallet.id, 1_000_000).await;
        let before = app.wallet_balance(token, wallet.id).await.available_msat;

        let amount_msat = 100_000_000u64;
        let bolt11 = Counterparty::for_provider(&app.provider).invoice(amount_msat, &unique("cp-invoice"));

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                SendPaymentRequest {
                    wallet_id: Some(wallet.id),
                    input: bolt11,
                    amount_msat: None,
                    comment: None,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let payment = res.parse::<Payment>();
        assert_eq!(
            payment.status,
            PaymentStatus::Settled,
            "payment not settled: {payment:?}"
        );
        assert_eq!(payment.amount_msat, amount_msat);

        // The wallet is debited by the amount plus the routing fee.
        let fee = payment.fee_msat.unwrap_or_default() as i64;
        let after = app.wallet_balance(token, wallet.id).await.available_msat;
        assert_eq!(
            after,
            before - amount_msat as i64 - fee,
            "wallet should be debited by amount + fee (before={before}, fee={fee})"
        );
    }
}
