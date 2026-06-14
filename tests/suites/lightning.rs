//! Real Lightning flows over the LND<->CLN channel. These reach the LN
//! provider, so they run across the provider matrix. Each test uses its own
//! wallet for balance isolation.

use std::time::Duration;

use reqwest::StatusCode;

use swissknife_types::{Invoice, NewInvoiceRequest, Payment, PaymentStatus, SendPaymentRequest};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::unique;
use crate::common::wait::wait_until;
use crate::common::{app, assert_error, assert_status, Auth};

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

mod failure {
    use super::*;

    /// A payment whose amount exceeds the only channel's capacity (5,000,000 sat)
    /// has no possible route, so the LN attempt fails. SwissKnife must mark it
    /// failed and release the reservation, leaving the wallet whole — the invariant
    /// the payment UoW enforces, exercised here over a real backend. Exceeding
    /// channel *capacity* (not just current liquidity) keeps this deterministic
    /// regardless of test order.
    #[tokio::test]
    async fn an_unroutable_payment_fails_and_releases_the_reservation() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "ln-unroutable").await;

        // Fund above channel capacity so the reserve passes and the failure is a
        // routing failure, not insufficient funds.
        app.fund_onchain(token, wallet.id, 10_000_000).await;
        let before = app.wallet_balance(token, wallet.id).await.available_msat;

        // 8M sat > 5M channel capacity: no route can carry it.
        let amount_msat = 8_000_000_000u64;
        let bolt11 = Counterparty::for_provider(&app.provider).invoice(amount_msat, &unique("cp-unroutable"));

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
        // A failed Lightning payment is a client-facing 422 (LightningError::Pay).
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);

        // The reservation is released: the wallet is back where it started.
        let after = app.wallet_balance(token, wallet.id).await.available_msat;
        assert_eq!(after, before, "the failed payment released its reservation");

        // And the attempt is recorded as a failed payment with a reason.
        let payments = app
            .api()
            .get(&format!("/v1/payments?wallet_id={}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&payments, StatusCode::OK);
        assert!(
            payments
                .parse::<Vec<Payment>>()
                .iter()
                .any(|p| p.status == PaymentStatus::Failed && p.error.is_some()),
            "the unroutable attempt is recorded as failed"
        );
    }
}
