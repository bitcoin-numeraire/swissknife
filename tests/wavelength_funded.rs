//! Opt-in funded public-network wallet round trip, without an operator backend.
//! The printed boarding address needs test coins. Wallet data survives exit.
#![cfg(feature = "funded-itest")]
#[allow(unused_imports)]
mod common;

use bitcoin::hashes::{sha256, Hash};
use common::{assert_status, fixtures::unique, harness::matrix_cell, wait::wait_until, Auth, TestApp};
use lightning_invoice::Bolt11Invoice;
use reqwest::StatusCode;
use std::{path::PathBuf, process::Command, str::FromStr, time::Duration};
use swissknife_types::{
    Invoice, InvoiceStatus, NewInvoiceRequest, Payment, PaymentFeeEstimate, PaymentStatus, SendPaymentRequest, Wallet,
};
use wavelength_client::{rpc, swaps, Client, Config};

// Fresh public-test-network wallets only; neither a real seed nor a password
// supplied by the operator is ever accepted by this runner.
const PASSWORD: &[u8] = b"swissknife-funded-public-test";
const FEE_CAP_SAT: u64 = 1_000;
const RECEIVE_MSAT: u64 = 50_000_000;
const SEND_MSAT: u64 = 21_123_000;
const SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(600);

struct WalletRpc {
    endpoint: String,
    credentials: PathBuf,
    client: Client,
}

impl WalletRpc {
    async fn connect(role: &'static str, port: &str, state: &std::path::Path) -> Self {
        let endpoint = format!("https://127.0.0.1:{port}");
        let credentials = state.join(role);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            let connection = Client::connect(Config {
                endpoint: endpoint.clone(),
                certificate_pem: std::fs::read(credentials.join("tls.cert")).expect("test certificate"),
                macaroon: std::fs::read(credentials.join("admin.macaroon")).expect("test macaroon"),
                connect_timeout: Duration::from_secs(3),
                request_timeout: Duration::from_secs(60),
            })
            .await;
            match connection {
                Ok(client) => {
                    return Self {
                        endpoint,
                        credentials,
                        client,
                    }
                }
                Err(error) if tokio::time::Instant::now() >= deadline => panic!("{role} RPC unavailable: {error}"),
                _ => tokio::time::sleep(Duration::from_millis(250)).await,
            }
        }
    }

    async fn initialize(&self, phase: &str, network: &str) {
        let identity = match phase {
            "create" => {
                let response = self
                    .client
                    .create(rpc::CreateRequest {
                        wallet_password: PASSWORD.to_vec(),
                        ..Default::default()
                    })
                    .await
                    .expect("create a fresh test wallet once");
                assert_eq!(response.mnemonic.len(), 24);
                // Never print, serialize or include this response in assertions.
                drop(response.mnemonic);
                std::fs::write(self.credentials.join("identity"), &response.identity_pubkey).unwrap();
                response.identity_pubkey
            }
            "resume" => {
                let response = self
                    .client
                    .unlock(rpc::UnlockRequest {
                        wallet_password: PASSWORD.to_vec(),
                    })
                    .await
                    .expect("unlock the existing test wallet once");
                assert_eq!(
                    response.identity_pubkey,
                    std::fs::read_to_string(self.credentials.join("identity")).unwrap()
                );
                response.identity_pubkey
            }
            _ => panic!("expected an explicit create/resume phase"),
        };
        assert_eq!(identity.len(), 66);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
        loop {
            let result = self.client.status(Default::default()).await;
            if let Ok(status) = &result {
                assert_eq!(status.network, network, "test-network isolation");
                if status.ready && status.unlocked {
                    return;
                }
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "public wallet readiness: {result:?}"
            );
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

async fn canonical_settlement(client: &Client, hash: &str, amount_msat: u64) -> swaps::SwapSummary {
    let deadline = tokio::time::Instant::now() + SETTLEMENT_TIMEOUT;
    loop {
        if let Ok(response) = client
            .get_swap(swaps::GetSwapRequest {
                payment_hash: hash.into(),
            })
            .await
        {
            if let Some(swap) = response.swap {
                assert_eq!(swap.payment_hash, hash);
                assert_eq!(swap.direction, swaps::SwapDirection::Pay as i32);
                if !swap.pending && swap.state == swaps::SwapState::Completed as i32 {
                    let invoice = Bolt11Invoice::from_str(&swap.invoice).expect("canonical invoice");
                    assert_eq!(invoice.payment_hash().to_string(), hash);
                    assert_eq!(invoice.amount_milli_satoshis(), Some(amount_msat));
                    let preimage = hex::decode(&swap.preimage).expect("settlement proof");
                    assert_eq!(preimage.len(), 32);
                    assert_eq!(sha256::Hash::hash(&preimage).to_string(), hash);
                    assert!(swap.fee_sat <= FEE_CAP_SAT, "final fee exceeds authorization");
                    return swap;
                }
                assert!(
                    !matches!(
                        swaps::SwapState::try_from(swap.state),
                        Ok(swaps::SwapState::Failed | swaps::SwapState::Expired | swaps::SwapState::Refunded)
                    ),
                    "public swap failed: {}",
                    swap.terminal_reason
                );
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no canonical settlement for {hash}; never resubmit this invoice automatically"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn pay_once(client: &Client, invoice: &str) -> swaps::SwapSummary {
    let parsed = Bolt11Invoice::from_str(invoice).unwrap();
    let principal = parsed.amount_milli_satoshis().unwrap() / 1000;
    let hash = parsed.payment_hash().to_string();
    let quote = client
        .prepare_send(rpc::PrepareSendRequest {
            destination: Some(rpc::prepare_send_request::Destination::Invoice(invoice.into())),
            max_fee_sat: FEE_CAP_SAT,
            // No operator-credit top-up or credit spending for this counterparty.
            max_credit_sat: 0,
            max_credit_topup_sat: 0,
            ..Default::default()
        })
        .await
        .expect("complete funded quote");
    assert_eq!(quote.payment_hash, hash);
    assert_eq!(quote.amount_sat, principal as i64);
    assert!(quote.fee_known && quote.total_outflow_known);
    assert_eq!(quote.quote_status, rpc::SendQuoteStatus::Complete as i32);
    assert!(quote.expected_fee_sat >= 0 && quote.expected_fee_sat as u64 <= FEE_CAP_SAT);
    assert_eq!(
        quote.expected_total_outflow_sat,
        principal as i64 + quote.expected_fee_sat
    );
    assert!(!quote.send_intent_id.is_empty());
    let reply = client
        .send(rpc::SendRequest {
            send_intent_id: quote.send_intent_id,
        })
        .await;
    // A lost reply does not authorize another dispatch. Only read by hash.
    if let Ok(reply) = reply {
        eprintln!("Counterparty Send.actual_amount_sat={}", reply.actual_amount_sat);
    }
    canonical_settlement(client, &hash, principal * 1000).await
}

fn restart_wallets(project: &str) {
    assert!(project.starts_with("swissknife-wave-funded-wavelength-funded-"));
    let result = Command::new("docker")
        .args([
            "compose",
            "-p",
            project,
            "-f",
            "tests/itest/docker-compose.yml",
            "-f",
            "tests/itest/docker-compose.wavelength-funded.yml",
            "restart",
            "waved-funded-payer",
            "waved-funded-provider",
        ])
        .status()
        .expect("restart this test's public client daemons");
    assert!(result.success());
}

#[tokio::test]
async fn public_funded_receive_offline_and_outgoing_payment_replay_once_after_restart() {
    let state = PathBuf::from(std::env::var("WAVELENGTH_FUNDED_STATE_DIR").expect("run the funded shell runner"));
    let network = std::env::var("WAVELENGTH_PUBLIC_NETWORK").expect("test network");
    assert!(matches!(network.as_str(), "signet" | "testnet"));
    let phase = std::env::var("WAVELENGTH_FUNDED_PHASE").unwrap();
    let payer_port = std::env::var("WAVELENGTH_FUNDED_PAYER_PORT").unwrap();
    let provider_port = std::env::var("WAVELENGTH_FUNDED_PROVIDER_PORT").unwrap();
    let payer = WalletRpc::connect("payer", &payer_port, &state).await;
    let provider = WalletRpc::connect("provider", &provider_port, &state).await;
    payer.initialize(&phase, &network).await;
    provider.initialize(&phase, &network).await;

    let deposit_file = state.join("funding-address");
    let address = if deposit_file.exists() {
        std::fs::read_to_string(&deposit_file).unwrap()
    } else {
        let deposit = payer
            .client
            .deposit(Default::default())
            .await
            .expect("test boarding address");
        assert!(deposit.onchain_address.starts_with("tb1"));
        std::fs::write(&deposit_file, &deposit.onchain_address).unwrap();
        deposit.onchain_address
    };
    let env = vec![
        ("SWISSKNIFE_INVOICE_EXPIRY", "provider".into()),
        ("SWISSKNIFE_WAVELENGTH_CONFIG__ENDPOINT", provider.endpoint.clone()),
        (
            "SWISSKNIFE_WAVELENGTH_CONFIG__CERT_PATH",
            provider.credentials.join("tls.cert").display().to_string(),
        ),
        (
            "SWISSKNIFE_WAVELENGTH_CONFIG__MACAROON_PATH",
            provider.credentials.join("admin.macaroon").display().to_string(),
        ),
        (
            "SWISSKNIFE_WAVELENGTH_CONFIG__FEE_LIMIT_MSAT",
            (FEE_CAP_SAT * 1000).to_string(),
        ),
        ("SWISSKNIFE_WAVELENGTH_CONFIG__ALLOW_SERVER_CREDIT", "true".into()),
        ("SWISSKNIFE_WAVELENGTH_CONFIG__REQUEST_TIMEOUT", "60s".into()),
        ("SWISSKNIFE_WAVELENGTH_CONFIG__PAYMENT_TIMEOUT", "1ms".into()),
    ];
    let mut app = TestApp::isolated_for(&matrix_cell().0, "wavelength", &unique("wave-funded"), &env).await;
    let token = app.admin_token().await.to_owned();
    let wallets = app
        .api()
        .get("/v1/me/wallets", Auth::Bearer(&token))
        .await
        .parse::<Vec<Wallet>>();
    assert_eq!(wallets.len(), 1);
    let wallet_id = wallets[0].id;
    assert_eq!(app.wallet_balance(&token, wallet_id).await.available_msat, 0);
    let response = app
        .api()
        .post(
            "/v1/invoices",
            Auth::Bearer(&token),
            NewInvoiceRequest {
                wallet_id: Some(wallet_id),
                amount_msat: RECEIVE_MSAT,
                description: Some("public funded offline receive".into()),
                expiry: None,
            },
        )
        .await;
    assert_status(&response, StatusCode::OK);
    let incoming = response.parse::<Invoice>();
    // Verify receive allocation through the public operator before requesting
    // funding; a protocol mismatch must fail before a new user funds this run.
    app.stop();
    eprintln!("FUNDING_NETWORK={network}\nFUNDING_ADDRESS={address}\nFund the payer with at least 100000 test sats; waiting for spendable VTXOs (20 minutes). State: {}", state.display());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1200);
    let mut last_balance = None;
    loop {
        let result = payer.client.balance(Default::default()).await;
        if let Ok(balance) = &result {
            let snapshot = (balance.confirmed_sat, balance.pending_in_sat, balance.pending_out_sat);
            if last_balance != Some(snapshot) {
                eprintln!(
                    "Payer VTXO balance: confirmed={} pending_in={} pending_out={} sats",
                    snapshot.0, snapshot.1, snapshot.2
                );
                last_balance = Some(snapshot);
            }
            if balance.confirmed_sat >= 100_000 {
                break;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "funded boarding did not complete; final balance: {result:?}; test state retained at {}",
            state.display()
        );
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    let external_payment = pay_once(&payer.client, &incoming.ln_invoice.as_ref().unwrap().bolt11).await;
    eprintln!("Funded incoming counterparty fee: {} sats", external_payment.fee_sat);
    app = app.restart(&unique("wave-funded-incoming-replay"), &env).await;
    wait_until(SETTLEMENT_TIMEOUT, "offline receive credited once", || async {
        app.wallet_balance(&token, wallet_id).await.available_msat == RECEIVE_MSAT as i64
    })
    .await;
    let received = app
        .api()
        .get(&format!("/v1/invoices/{}", incoming.id), Auth::Bearer(&token))
        .await
        .parse::<Invoice>();
    assert_eq!(received.status, InvoiceStatus::Settled);
    assert_eq!(received.amount_received_msat, Some(RECEIVE_MSAT));

    let destination = payer
        .client
        .recv(rpc::RecvRequest {
            amt_sat: SEND_MSAT / 1000,
            memo: "public funded outgoing".into(),
        })
        .await
        .expect("public counterparty receive");
    let hash = Bolt11Invoice::from_str(&destination.invoice)
        .unwrap()
        .payment_hash()
        .to_string();
    let request = SendPaymentRequest {
        wallet_id: Some(wallet_id),
        input: destination.invoice,
        amount_msat: None,
        comment: None,
    };
    let estimate = app
        .api()
        .post("/v1/payments/fee-estimate", Auth::Bearer(&token), &request)
        .await;
    assert_status(&estimate, StatusCode::OK);
    let estimate = estimate.parse::<PaymentFeeEstimate>();
    assert_eq!(estimate.amount_msat, SEND_MSAT);
    assert_eq!(estimate.maximum_fee_msat, FEE_CAP_SAT * 1000);
    assert!(estimate
        .estimated_fee_msat
        .is_some_and(|fee| fee <= estimate.maximum_fee_msat));
    let sent = app.api().post("/v1/payments", Auth::Bearer(&token), request).await;
    assert_status(&sent, StatusCode::OK);
    let sent = sent.parse::<Payment>();
    assert!(matches!(sent.status, PaymentStatus::Pending | PaymentStatus::Settled));
    app.stop();
    let canonical = canonical_settlement(&provider.client, &hash, SEND_MSAT).await;
    let expected = (RECEIVE_MSAT - SEND_MSAT - canonical.fee_sat * 1000) as i64;
    app = app.restart(&unique("wave-funded-outgoing-replay"), &env).await;
    wait_until(SETTLEMENT_TIMEOUT, "outgoing canonical settlement replay", || async {
        let balance = app.wallet_balance(&token, wallet_id).await;
        balance.available_msat == expected && balance.reserved_msat == 0
    })
    .await;
    let payment = app
        .api()
        .get(&format!("/v1/payments/{}", sent.id), Auth::Bearer(&token))
        .await
        .parse::<Payment>();
    assert_eq!(payment.status, PaymentStatus::Settled);
    assert_eq!(payment.amount_msat, SEND_MSAT);
    assert_eq!(payment.fee_msat, Some(canonical.fee_sat * 1000));
    assert_eq!(
        payment.lightning.unwrap().payment_preimage.as_deref(),
        Some(canonical.preimage.as_str())
    );
    eprintln!(
        "Funded SwissKnife outgoing fee: {} sats; settlement rail={}",
        canonical.fee_sat, canonical.settlement_type
    );

    wait_until(SETTLEMENT_TIMEOUT, "counterparty receives the outgoing amount", || async {
        payer.client.activity(100, 100).await.is_ok_and(|entries| entries.iter().any(|entry| {
            entry.kind == rpc::EntryKind::Recv as i32
                && entry.status == rpc::EntryStatus::Complete as i32
                && entry.amount_sat == (SEND_MSAT / 1000) as i64
                && (entry.progress.as_ref().is_some_and(|p| p.payment_hash == hash)
                    || entry.request.as_ref().and_then(|r| r.request.as_ref()).is_some_and(|r| {
                        matches!(r, rpc::wallet_entry_request::Request::LightningInvoice(invoice) if invoice.payment_hash == hash)
                    }))
        }))
    }).await;

    app.stop();
    restart_wallets(&std::env::var("WAVELENGTH_FUNDED_PROJECT").unwrap());
    let payer = WalletRpc::connect("payer", &payer_port, &state).await;
    let provider = WalletRpc::connect("provider", &provider_port, &state).await;
    payer.initialize("resume", &network).await;
    provider.initialize("resume", &network).await;
    assert_eq!(
        canonical_settlement(&provider.client, &hash, SEND_MSAT).await.fee_sat,
        canonical.fee_sat
    );
    app = app.restart(&unique("wave-funded-daemon-replay"), &env).await;
    let balance = app.wallet_balance(&token, wallet_id).await;
    assert_eq!(balance.available_msat, expected);
    assert_eq!(balance.reserved_msat, 0);
    let payments = app
        .api()
        .get(&format!("/v1/payments?wallet_id={wallet_id}"), Auth::Bearer(&token))
        .await
        .parse::<Vec<Payment>>();
    assert_eq!(
        payments
            .iter()
            .filter(|p| p.lightning.as_ref().is_some_and(|ln| ln.payment_hash == hash))
            .count(),
        1
    );
    app.shutdown();
}
