//! Black-box SwissKnife API tests against an independent public-RPC simulator.
//! These prove accounting/failure boundaries, not funded Ark interoperability.
#![cfg(feature = "itest")]
#[allow(unused_imports)]
mod common;
#[path = "support/wavelength.rs"]
mod daemon;

use common::{assert_status, fixtures::unique, harness::matrix_cell, wait::wait_until, Auth, TestApp};
use daemon::{Daemon, Outcome, Quote};
use reqwest::StatusCode;
use std::{path::PathBuf, time::Duration};
use swissknife_types::{
    Invoice, InvoiceStatus, NewInvoiceRequest, Payment, PaymentStatus, Permission, RegisterLnAddressRequest,
    SendPaymentRequest,
};

struct Fixture {
    app: TestApp,
    daemon: Daemon,
    task: tokio::task::JoinHandle<()>,
    credentials: PathBuf,
    env: Vec<(&'static str, String)>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.app.shutdown();
        self.task.abort();
        std::fs::remove_dir_all(&self.credentials).unwrap();
    }
}
impl Fixture {
    async fn new(outcome: Outcome, quote: Quote) -> Self {
        let daemon = Daemon::default();
        {
            let mut state = daemon.0.lock().unwrap();
            state.outcome = outcome;
            state.quote = quote;
        }
        let (endpoint, task) = daemon.serve().await;
        let credentials = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/itest")
            .join(unique("wave-rpc"));
        std::fs::create_dir_all(&credentials).unwrap();
        std::fs::write(credentials.join("tls.cert"), daemon::CERT).unwrap();
        std::fs::write(credentials.join("admin.macaroon"), daemon::TOKEN).unwrap();
        let env = vec![
            ("SWISSKNIFE_INVOICE_EXPIRY", "provider".into()),
            ("SWISSKNIFE_WAVELENGTH_CONFIG__ENDPOINT", endpoint),
            (
                "SWISSKNIFE_WAVELENGTH_CONFIG__CERT_PATH",
                credentials.join("tls.cert").display().to_string(),
            ),
            (
                "SWISSKNIFE_WAVELENGTH_CONFIG__MACAROON_PATH",
                credentials.join("admin.macaroon").display().to_string(),
            ),
            ("SWISSKNIFE_WAVELENGTH_CONFIG__REQUEST_TIMEOUT", "2s".into()),
            ("SWISSKNIFE_WAVELENGTH_CONFIG__PAYMENT_TIMEOUT", "300ms".into()),
            ("SWISSKNIFE_WAVELENGTH_CONFIG__POLL_INTERVAL", "50ms".into()),
            ("SWISSKNIFE_WAVELENGTH_CONFIG__ACTIVITY_PAGE_SIZE", "1".into()),
            ("SWISSKNIFE_WAVELENGTH_CONFIG__ALLOW_SERVER_CREDIT", "true".into()),
        ];
        let app = TestApp::isolated_for(&matrix_cell().0, "wavelength", &unique("wave-contract"), &env).await;
        Self {
            app,
            daemon,
            task,
            credentials,
            env,
        }
    }
    async fn receive(&self, wallet_id: uuid::Uuid, amount_msat: u64) -> Invoice {
        let response = self
            .app
            .api()
            .post(
                "/v1/invoices",
                Auth::Bearer(self.app.admin_token().await),
                NewInvoiceRequest {
                    wallet_id: Some(wallet_id),
                    amount_msat,
                    description: Some("wallet contract".into()),
                    expiry: None,
                },
            )
            .await;
        assert_status(&response, StatusCode::OK);
        response.parse()
    }
    async fn funded_wallet(&self) -> swissknife_types::Wallet {
        let token = self.app.admin_token().await;
        let wallet = self.app.create_wallet(token, "payer").await;
        let invoice = self.receive(wallet.id, 100_000_000).await;
        self.daemon.complete(&invoice.ln_invoice.unwrap().payment_hash, None, 0);
        wait_until(
            Duration::from_secs(5),
            "simulated incoming payment credited",
            || async { self.app.wallet_balance(token, wallet.id).await.available_msat == 100_000_000 },
        )
        .await;
        wallet
    }
    async fn pay(&self, wallet_id: uuid::Uuid) -> common::client::TestResponse {
        let invoice = daemon::invoice(21_123_000, "external recipient", [21; 32]);
        self.app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(self.app.admin_token().await),
                SendPaymentRequest {
                    wallet_id: Some(wallet_id),
                    input: invoice.to_string(),
                    amount_msat: None,
                    comment: None,
                },
            )
            .await
    }
}

#[tokio::test]
async fn ordinary_receive_uses_daemon_expiry_and_unsupported_options_fail_before_rpc() {
    let f = Fixture::new(Outcome::LostReply, Quote::Complete).await;
    let token = f.app.admin_token().await;
    let wallet = f.app.create_wallet(token, "receives").await;
    let invoice = f.receive(wallet.id, 21_000_000).await;
    assert_eq!(invoice.ln_invoice.unwrap().expiry, Duration::from_secs(3600));
    for (amount, expiry) in [(21_000_000, Some(60)), (21_000_001, None), (0, None)] {
        let response = f
            .app
            .api()
            .post(
                "/v1/invoices",
                Auth::Bearer(token),
                NewInvoiceRequest {
                    wallet_id: Some(wallet.id),
                    amount_msat: amount,
                    description: None,
                    expiry,
                },
            )
            .await;
        assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    }
    assert_eq!(f.daemon.0.lock().unwrap().receives, 1);
    let response = f
        .app
        .api()
        .post(
            "/v1/bitcoin/addresses",
            Auth::Bearer(token),
            serde_json::json!({"wallet_id":wallet.id}),
        )
        .await;
    assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    let username = unique("wave-lnurl");
    let response = f
        .app
        .api()
        .post(
            "/v1/lightning-addresses",
            Auth::Bearer(token),
            RegisterLnAddressRequest {
                account_id: Some(wallet.account_id),
                username: username.clone(),
                allows_nostr: false,
                nostr_pubkey: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::OK);
    let response = f
        .app
        .api()
        .get(&format!("/lnurlp/{username}/callback?amount=21000000"), Auth::None)
        .await;
    assert!(response.status == StatusCode::UNPROCESSABLE_ENTITY || response.body["status"] == "ERROR");
    assert_eq!(f.daemon.0.lock().unwrap().receives, 1);
    let response = f
        .app
        .api()
        .post("/v1/invoices", Auth::None, serde_json::json!({}))
        .await;
    assert_status(&response, StatusCode::UNAUTHORIZED);
    let key = f.app.api_key(token, vec![Permission::ReadTransaction]).await;
    let response = f
        .app
        .api()
        .post(
            "/v1/invoices",
            Auth::ApiKey(&key),
            NewInvoiceRequest {
                wallet_id: Some(wallet.id),
                amount_msat: 21_000_000,
                description: None,
                expiry: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn receives_with_non_hash_ids_replay_after_restart_and_credit_once() {
    let f = Fixture::new(Outcome::LostReply, Quote::Complete).await;
    let token = f.app.admin_token().await;
    let wallet = f.app.create_wallet(token, "offline receive").await;
    let invoice = f.receive(wallet.id, 21_000_000).await;
    let hash = invoice.ln_invoice.as_ref().unwrap().payment_hash.clone();
    assert_ne!(f.daemon.0.lock().unwrap().entries[0].id, hash);
    f.app.stop();
    f.daemon.complete(&hash, None, 0);
    let restarted = f.app.restart(&unique("wave-recv-restart"), &f.env).await;
    wait_until(Duration::from_secs(5), "offline invoice replay", || async {
        restarted.wallet_balance(token, wallet.id).await.available_msat == 21_000_000
    })
    .await;
    let again = restarted.restart(&unique("wave-recv-twice"), &f.env).await;
    assert_eq!(again.wallet_balance(token, wallet.id).await.available_msat, 21_000_000);
    let got = again
        .api()
        .get(&format!("/v1/invoices/{}", invoice.id), Auth::Bearer(token))
        .await
        .parse::<Invoice>();
    assert_eq!(got.status, InvoiceStatus::Settled);
    again.shutdown();
}

#[tokio::test]
async fn gross_outflow_with_nonzero_fee_and_proof_is_debited_exactly_once() {
    let f = Fixture::new(Outcome::Complete, Quote::Complete).await;
    let wallet = f.funded_wallet().await;
    let invoice = daemon::invoice(21_123_000, "external recipient", [21; 32]);
    let quote = f
        .app
        .api()
        .post(
            "/v1/payments/fee-estimate",
            Auth::Bearer(f.app.admin_token().await),
            SendPaymentRequest {
                wallet_id: Some(wallet.id),
                input: invoice.to_string(),
                amount_msat: None,
                comment: None,
            },
        )
        .await;
    assert_status(&quote, StatusCode::OK);
    assert_eq!(quote.body["estimated_fee_msat"], 2_000);
    assert_eq!(f.daemon.0.lock().unwrap().prepares.len(), 0);
    let response = f.pay(wallet.id).await;
    assert_status(&response, StatusCode::OK);
    let payment = response.parse::<Payment>();
    assert_eq!(payment.status, PaymentStatus::Settled);
    assert_eq!(payment.amount_msat, 21_123_000);
    assert_eq!(payment.fee_msat, Some(2_000));
    assert_eq!(payment.lightning.unwrap().payment_preimage, Some(hex::encode([21; 32])));
    let balance = f.app.wallet_balance(f.app.admin_token().await, wallet.id).await;
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(balance.available_msat, 78_875_000);
    assert_eq!(f.daemon.0.lock().unwrap().sends, 1);
}

#[tokio::test]
async fn credit_poll_timeout_waits_for_server_debit_and_uses_the_persisted_swap_fee() {
    let f = Fixture::new(Outcome::WalletTimeout, Quote::CreditOnly).await;
    let wallet = f.funded_wallet().await;
    let response = f.pay(wallet.id).await;
    assert_status(&response, StatusCode::OK);
    assert_eq!(response.parse::<Payment>().status, PaymentStatus::Pending);
    let token = f.app.admin_token().await;
    assert_eq!(f.app.wallet_balance(token, wallet.id).await.reserved_msat, 21_148_000);
    f.app.stop();
    f.daemon.0.lock().unwrap().credit_settled = true;
    let restarted = f.app.restart(&unique("wave-credit-replay"), &f.env).await;
    wait_until(Duration::from_secs(5), "authoritative credit debit replay", || async {
        restarted.wallet_balance(token, wallet.id).await.reserved_msat == 0
    })
    .await;
    let balance = restarted.wallet_balance(token, wallet.id).await;
    assert_eq!(balance.available_msat, 78_875_000);
    assert_eq!(balance.fees_paid_msat, 2_000);
    assert_eq!(f.daemon.0.lock().unwrap().sends, 1);
    restarted.shutdown();
}

#[tokio::test]
async fn incomplete_inconsistent_and_over_budget_quotes_never_dispatch_or_debit() {
    for quote in [Quote::Partial, Quote::WrongHash, Quote::OverBudget, Quote::CreditTopup] {
        let f = Fixture::new(Outcome::Complete, quote).await;
        let wallet = f.funded_wallet().await;
        let response = f.pay(wallet.id).await;
        assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
        let balance = f.app.wallet_balance(f.app.admin_token().await, wallet.id).await;
        assert_eq!(balance.available_msat, 100_000_000);
        assert_eq!(balance.reserved_msat, 0);
        assert_eq!(f.daemon.0.lock().unwrap().sends, 0);
    }
}

#[tokio::test]
async fn a_malformed_read_only_quote_is_rejected_before_reservation() {
    let f = Fixture::new(Outcome::Complete, Quote::ReadOnlyWrongHash).await;
    let wallet = f.funded_wallet().await;
    let response = f.pay(wallet.id).await;
    assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    let payments = f
        .app
        .api()
        .get(
            &format!("/v1/payments?wallet_id={}", wallet.id),
            Auth::Bearer(f.app.admin_token().await),
        )
        .await
        .parse::<Vec<Payment>>();
    assert!(payments.is_empty());
    assert_eq!(f.daemon.0.lock().unwrap().prepares.len(), 0);
    assert_eq!(f.daemon.0.lock().unwrap().sends, 0);
    assert_eq!(
        f.app
            .wallet_balance(f.app.admin_token().await, wallet.id)
            .await
            .available_msat,
        100_000_000
    );
}

#[tokio::test]
async fn lost_reply_wallet_timeout_bad_proof_and_fee_overrun_keep_reservations() {
    for outcome in [
        Outcome::LostReply,
        Outcome::WalletTimeout,
        Outcome::BadProof,
        Outcome::OverBudget,
    ] {
        let f = Fixture::new(outcome, Quote::Complete).await;
        let wallet = f.funded_wallet().await;
        let response = f.pay(wallet.id).await;
        assert_status(&response, StatusCode::OK);
        let payment = response.parse::<Payment>();
        assert_eq!(payment.status, PaymentStatus::Pending);
        tokio::time::sleep(Duration::from_millis(200)).await;
        let balance = f.app.wallet_balance(f.app.admin_token().await, wallet.id).await;
        assert_eq!(balance.reserved_msat, 21_148_000);
        assert_eq!(balance.available_msat, 78_852_000);
        assert_eq!(f.daemon.0.lock().unwrap().sends, 1);
    }
}

#[tokio::test]
async fn lost_send_reconciles_after_restart_without_preparing_or_sending_again() {
    let f = Fixture::new(Outcome::LostReply, Quote::Complete).await;
    let wallet = f.funded_wallet().await;
    let response = f.pay(wallet.id).await;
    assert_status(&response, StatusCode::OK);
    let payment = response.parse::<Payment>();
    let token = f.app.admin_token().await;
    f.app.stop();
    let hash = &payment.lightning.as_ref().unwrap().payment_hash;
    f.daemon.complete(hash, Some([21; 32]), 2);
    // Daemon intents are volatile. Durable activity is sufficient for recovery.
    f.daemon.0.lock().unwrap().intents_clear();
    let restarted = f.app.restart(&unique("wave-send-restart"), &f.env).await;
    wait_until(Duration::from_secs(5), "lost send replay", || async {
        restarted.wallet_balance(token, wallet.id).await.reserved_msat == 0
    })
    .await;
    let again = restarted.restart(&unique("wave-send-twice"), &f.env).await;
    assert_eq!(again.wallet_balance(token, wallet.id).await.available_msat, 78_875_000);
    let state = f.daemon.0.lock().unwrap();
    assert_eq!(state.sends, 1);
    assert_eq!(state.prepares.len(), 1);
    drop(state);
    again.shutdown();
}

#[tokio::test]
async fn explicit_refund_releases_the_reservation_without_a_debit() {
    let f = Fixture::new(Outcome::Refunded, Quote::Complete).await;
    let wallet = f.funded_wallet().await;
    let response = f.pay(wallet.id).await;
    assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    let balance = f.app.wallet_balance(f.app.admin_token().await, wallet.id).await;
    assert_eq!(balance.available_msat, 100_000_000);
    assert_eq!(balance.reserved_msat, 0);
}

#[tokio::test]
async fn a_persisted_swap_missing_from_activity_is_not_dispatched_again() {
    let f = Fixture::new(Outcome::LostReply, Quote::Complete).await;
    let wallet = f.funded_wallet().await;
    let invoice = daemon::invoice(21_123_000, "external recipient", [21; 32]);
    let hash = invoice.payment_hash().to_string();
    f.daemon.record_existing_send(&invoice);
    let token = f.app.admin_token().await;
    // Unrelated provider history does not create an outgoing SwissKnife debit.
    assert_eq!(f.app.wallet_balance(token, wallet.id).await.available_msat, 100_000_000);
    let response = f.pay(wallet.id).await;
    assert_status(&response, StatusCode::OK);
    assert_eq!(response.parse::<Payment>().status, PaymentStatus::Pending);
    assert_eq!(f.daemon.0.lock().unwrap().sends, 0);
    assert_eq!(f.daemon.0.lock().unwrap().prepares.len(), 0);
    f.app.stop();
    f.daemon.complete(&hash, Some([21; 32]), 2);
    let restarted = f.app.restart(&unique("wave-hidden-swap-replay"), &f.env).await;
    wait_until(
        Duration::from_secs(5),
        "canonical swap recovery without activity",
        || async { restarted.wallet_balance(token, wallet.id).await.reserved_msat == 0 },
    )
    .await;
    assert_eq!(
        restarted.wallet_balance(token, wallet.id).await.available_msat,
        78_875_000
    );
    assert_eq!(f.daemon.0.lock().unwrap().sends, 0);
    restarted.shutdown();
}

#[tokio::test]
async fn fractional_send_amounts_and_zero_fee_budgets_never_become_uncapped_sends() {
    let mut f = Fixture::new(Outcome::Complete, Quote::Complete).await;
    let wallet = f.funded_wallet().await;
    let invoice = daemon::invoice(21_123_001, "fractional satoshi", [21; 32]);
    let response = f
        .app
        .api()
        .post(
            "/v1/payments",
            Auth::Bearer(f.app.admin_token().await),
            SendPaymentRequest {
                wallet_id: Some(wallet.id),
                input: invoice.to_string(),
                amount_msat: None,
                comment: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    let mut env = f.env.clone();
    env.push(("SWISSKNIFE_WAVELENGTH_CONFIG__FEE_LIMIT_MSAT", "0".into()));
    f.app = f.app.restart(&unique("wave-zero-fee-cap"), &env).await;
    let response = f.pay(wallet.id).await;
    assert_status(&response, StatusCode::UNPROCESSABLE_ENTITY);
    let balance = f.app.wallet_balance(f.app.admin_token().await, wallet.id).await;
    assert_eq!(balance.available_msat, 100_000_000);
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(f.daemon.0.lock().unwrap().prepares.len(), 0);
    assert_eq!(f.daemon.0.lock().unwrap().sends, 0);
}

#[tokio::test]
async fn restart_with_a_larger_configured_limit_cannot_expand_an_existing_authorization() {
    let f = Fixture::new(Outcome::OverBudget, Quote::Complete).await;
    let wallet = f.funded_wallet().await;
    let response = f.pay(wallet.id).await;
    assert_status(&response, StatusCode::OK);
    assert_eq!(response.parse::<Payment>().status, PaymentStatus::Pending);
    f.app.stop();
    let mut env = f.env.clone();
    env.push(("SWISSKNIFE_WAVELENGTH_CONFIG__FEE_LIMIT_MSAT", "100000".into()));
    let restarted = f.app.restart(&unique("wave-original-budget"), &env).await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let balance = restarted.wallet_balance(f.app.admin_token().await, wallet.id).await;
    assert_eq!(balance.reserved_msat, 21_148_000);
    assert_eq!(balance.available_msat, 78_852_000);
    restarted.shutdown();
}
