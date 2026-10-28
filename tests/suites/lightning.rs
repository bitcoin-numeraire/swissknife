//! Real Lightning flows over the LND<->CLN channel. These reach the LN
//! provider, so they run across the provider matrix. Each test uses its own
//! wallet for balance isolation.

use std::time::Duration;

use reqwest::StatusCode;

use swissknife_types::{
    Invoice, InvoiceStatus, Ledger, NewInvoiceRequest, Payment, PaymentFeeEstimate, PaymentStatus, SendPaymentRequest,
};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::unique;
use crate::common::wait::wait_until;
use crate::common::{app, assert_error, assert_status, Auth, TestApp};

mod receive {
    use super::*;

    #[tokio::test]
    async fn a_payment_received_while_stopped_is_credited_on_restart() {
        let app = TestApp::isolated("ln-offline-receive", &[]).await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "ln-offline-receive").await;
        let amount_msat = 100_000_000;
        let created = app
            .api()
            .post(
                "/v1/invoices",
                Auth::Bearer(token),
                NewInvoiceRequest {
                    wallet_id: Some(wallet.id),
                    amount_msat,
                    description: Some("received while stopped".to_string()),
                    expiry: None,
                },
            )
            .await;
        assert_status(&created, StatusCode::OK);
        let invoice = created.parse::<Invoice>();
        let bolt11 = &invoice.ln_invoice.as_ref().unwrap().bolt11;

        app.stop();
        Counterparty::for_provider(&app.provider).pay(bolt11);
        let restarted = app.restart("ln-receive-replayed", &[]).await;
        wait_until(Duration::from_secs(45), "offline invoice credit replayed", || async {
            restarted.wallet_balance(token, wallet.id).await.available_msat == amount_msat as i64
        })
        .await;

        let persisted = restarted
            .api()
            .get(&format!("/v1/invoices/{}", invoice.id), Auth::Bearer(token))
            .await;
        assert_status(&persisted, StatusCode::OK);
        let persisted = persisted.parse::<Invoice>();
        assert_eq!(persisted.id, invoice.id);
        assert_eq!(persisted.wallet_id, wallet.id);
        assert_eq!(persisted.status, InvoiceStatus::Settled);
        assert_eq!(persisted.amount_msat, Some(amount_msat));
        assert_eq!(persisted.amount_received_msat, Some(amount_msat));
    }

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

        let quote = app
            .api()
            .post(
                "/v1/payments/fee-estimate",
                Auth::Bearer(token),
                SendPaymentRequest {
                    wallet_id: Some(wallet.id),
                    input: bolt11.clone(),
                    amount_msat: None,
                    comment: None,
                },
            )
            .await;
        assert_status(&quote, StatusCode::OK);
        let quote = quote.parse::<PaymentFeeEstimate>();
        assert_eq!(quote.ledger, Ledger::Lightning);
        assert_eq!(quote.amount_msat, amount_msat);
        let estimated_fee_msat = quote
            .estimated_fee_msat
            .expect("the real Lightning provider returns a route estimate");
        assert!(estimated_fee_msat <= quote.maximum_fee_msat);
        assert_eq!(quote.estimated_total_msat, Some(amount_msat + estimated_fee_msat));
        assert_eq!(quote.maximum_total_msat, amount_msat + quote.maximum_fee_msat);

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
        let fee = payment
            .fee_msat
            .expect("successful sends record the actual fee, including zero") as i64;
        assert!(
            fee as u64 <= quote.maximum_fee_msat,
            "settled fee stays within the quoted cap"
        );
        let balance = app.wallet_balance(token, wallet.id).await;
        assert_eq!(balance.reserved_msat, 0, "settlement releases the full reservation");
        let after = balance.available_msat;
        assert_eq!(
            after,
            before - amount_msat as i64 - fee,
            "wallet should be debited by amount + fee (before={before}, fee={fee})"
        );
        let persisted = app
            .api()
            .get(&format!("/v1/payments/{}", payment.id), Auth::Bearer(token))
            .await;
        assert_status(&persisted, StatusCode::OK);
        let persisted = persisted.parse::<Payment>();
        assert_eq!(persisted.id, payment.id);
        assert_eq!(persisted.wallet_id, wallet.id);
        assert_eq!(persisted.status, PaymentStatus::Settled);
        assert_eq!(persisted.amount_msat, amount_msat);
        assert_eq!(persisted.fee_msat, payment.fee_msat);
    }
}

mod failure {
    use super::*;

    #[tokio::test]
    async fn a_local_lnd_send_timeout_keeps_the_payment_and_reservation_pending() {
        if crate::common::harness::matrix_cell().1 != "lnd_grpc" {
            return;
        }
        let app = TestApp::isolated(
            "ln-send-timeout",
            &[("SWISSKNIFE_LND_GRPC_CONFIG__PAYMENT_TIMEOUT", "0s".to_string())],
        )
        .await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "ln-timeout").await;
        app.fund_onchain(token, wallet.id, 10_000_000).await;
        let before = app.wallet_balance(token, wallet.id).await.available_msat;
        let amount_msat = 8_000_000_000;
        let bolt11 = Counterparty::for_provider(&app.provider).invoice(amount_msat, &unique("cp-timeout"));
        let request = SendPaymentRequest {
            wallet_id: Some(wallet.id),
            input: bolt11,
            amount_msat: None,
            comment: None,
        };
        let quote = app
            .api()
            .post("/v1/payments/fee-estimate", Auth::Bearer(token), request.clone())
            .await;
        assert_status(&quote, StatusCode::OK);
        let quote = quote.parse::<PaymentFeeEstimate>();

        // This is a local establishment timeout, not proof of an accepted or in-flight HTLC.
        let response = app.api().post("/v1/payments", Auth::Bearer(token), request).await;
        assert_status(&response, StatusCode::OK);
        let payment = response.parse::<Payment>();
        assert_eq!(payment.status, PaymentStatus::Pending);
        assert_eq!(payment.wallet_id, wallet.id);
        assert_eq!(payment.amount_msat, amount_msat);
        assert!(payment.error.is_none());
        let balance = app.wallet_balance(token, wallet.id).await;
        assert_eq!(balance.reserved_msat, quote.maximum_total_msat);
        assert_eq!(balance.available_msat, before - quote.maximum_total_msat as i64);
        let persisted = app
            .api()
            .get(&format!("/v1/payments/{}", payment.id), Auth::Bearer(token))
            .await;
        assert_status(&persisted, StatusCode::OK);
        let persisted = persisted.parse::<Payment>();
        assert_eq!(persisted.id, payment.id);
        assert_eq!(persisted.status, PaymentStatus::Pending);
        assert_eq!(persisted.amount_msat, amount_msat);
        assert_eq!(
            persisted.lightning.as_ref().unwrap().payment_hash,
            payment.lightning.as_ref().unwrap().payment_hash
        );
    }

    /// Exceeding the only channel's capacity makes routing fail independently
    /// of current liquidity. LND exposes a terminal FAILED result; the CLN
    /// adapters cannot distinguish an xpay RPC error from a lost send result.
    #[tokio::test]
    async fn an_unroutable_payment_releases_funds_only_for_a_definitive_failure() {
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
        let request = SendPaymentRequest {
            wallet_id: Some(wallet.id),
            input: bolt11,
            amount_msat: None,
            comment: None,
        };
        let quote = app
            .api()
            .post("/v1/payments/fee-estimate", Auth::Bearer(token), request.clone())
            .await;
        assert_status(&quote, StatusCode::OK);
        let quote = quote.parse::<PaymentFeeEstimate>();

        let res = app.api().post("/v1/payments", Auth::Bearer(token), request).await;
        if app.provider.starts_with("cln_") {
            assert_status(&res, StatusCode::OK);
            let payment = res.parse::<Payment>();
            assert_eq!(payment.status, PaymentStatus::Pending);
            assert_eq!(payment.wallet_id, wallet.id);
            assert_eq!(payment.amount_msat, amount_msat);
            assert!(payment.error.is_none());
            let balance = app.wallet_balance(token, wallet.id).await;
            assert_eq!(balance.reserved_msat, quote.maximum_total_msat);
            assert_eq!(balance.available_msat, before - quote.maximum_total_msat as i64);
            let persisted = app
                .api()
                .get(&format!("/v1/payments/{}", payment.id), Auth::Bearer(token))
                .await;
            assert_status(&persisted, StatusCode::OK);
            let persisted = persisted.parse::<Payment>();
            assert_eq!(persisted.id, payment.id);
            assert_eq!(persisted.status, PaymentStatus::Pending);
            assert_eq!(persisted.amount_msat, amount_msat);
            assert_eq!(
                persisted.lightning.as_ref().unwrap().payment_hash,
                payment.lightning.as_ref().unwrap().payment_hash
            );
            return;
        }
        // LND's explicit terminal FAILED result remains a client-facing 422.
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
