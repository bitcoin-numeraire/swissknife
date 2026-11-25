//! Public, unauthenticated endpoints an external wallet hits to reach a
//! SwissKnife user: LNURL-pay (LUD-06/LUD-21) and Nostr NIP-05. These serve OUR
//! endpoints — no outbound calls — so no mock server is needed, and the LNURL
//! pay flow settles over the real LND<->CLN channel. Paying *out* to someone
//! else's address/Nostr (which needs a mock server) is covered separately.

use std::time::Duration;

use bitcoin::hashes::{sha256, Hash};
use lightning_invoice::Bolt11Invoice;
use reqwest::{header::CACHE_CONTROL, StatusCode};
use serde_json::{json, Value};

use swissknife_types::{
    Invoice, Ledger, LnAddress, LnURLPayRequest, LnUrlCallback, NewInvoiceRequest, NostrNIP05Response, Payment,
    SendPaymentRequest, UpdateLnAddressRequest, Wallet,
};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::unique;
use crate::common::wait::wait_until;
use crate::common::{app, assert_error, assert_status, Auth, TestApp};

/// Register an LN address (and optionally a Nostr key) under a fresh wallet,
/// returning the wallet and the created address.
async fn register_address(app: &TestApp, token: &str, label: &str, nostr_pubkey: Option<&str>) -> (Wallet, LnAddress) {
    let wallet = app.create_wallet(token, label).await;
    let body = serde_json::json!({
        "account_id": wallet.account_id,
        "username": unique(label),
        "allows_nostr": nostr_pubkey.is_some(),
        "nostr_pubkey": nostr_pubkey,
    });
    let res = app
        .api()
        .post("/v1/lightning-addresses", Auth::Bearer(token), body)
        .await;
    assert_status(&res, StatusCode::OK);
    (wallet, res.parse::<LnAddress>())
}

mod lnurl {
    use super::*;

    #[tokio::test]
    async fn configured_receive_identity_resolves_the_registered_address() {
        let app = TestApp::isolated(
            "public-identity",
            &[
                ("SWISSKNIFE_DOMAIN", "wallet.example".into()),
                ("SWISSKNIFE_HOST", "https://service.example/".into()),
            ],
        )
        .await;
        let token = app.admin_token().await;
        let (_, address) = register_address(&app, token, "identity", None).await;
        let res = app.api().get("/v1/system/receive-identity", Auth::None).await;
        assert_status(&res, StatusCode::OK);
        let identity = res.parse::<swissknife_types::ReceiveIdentity>();
        assert_eq!(identity.domain.as_deref(), Some("wallet.example"));
        let base = reqwest::Url::parse(identity.lnurl_base_url.as_deref().unwrap()).unwrap();
        assert_eq!(base.host_str(), Some("service.example"));
        let res = app
            .api()
            .get(&format!("{}{}", base.path(), address.username), Auth::None)
            .await;
        assert_status(&res, StatusCode::OK);
        let pay = res.parse::<LnURLPayRequest>();
        assert_eq!(
            pay.callback,
            format!("https://service.example/lnurlp/{}/callback", address.username)
        );
        assert!(pay.metadata.contains(&format!("{}@wallet.example", address.username)));
    }

    /// Follow the advertised LNURL `callback` URL exactly as an external wallet
    /// would, rather than reconstructing the path — so a wrong or unreachable
    /// advertised callback would be caught.
    async fn follow_callback(callback: &str, amount_msat: u64) -> LnUrlCallback {
        let res = reqwest::get(format!("{callback}?amount={amount_msat}"))
            .await
            .expect("reach the advertised LNURL callback");
        assert_eq!(
            res.status().as_u16(),
            200,
            "advertised callback was not reachable / failed"
        );
        assert_eq!(res.headers().get(CACHE_CONTROL).unwrap(), "no-store");
        res.json::<LnUrlCallback>()
            .await
            .expect("callback returns an LnUrlCallback")
    }

    async fn verify(url: &str) -> Value {
        let response = reqwest::get(url).await.expect("reach the LNURL verification endpoint");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-store");
        response.json().await.expect("verification returns JSON")
    }

    fn assert_payment_proof(response: &Value, pr: &str) {
        let preimage = hex::decode(response["preimage"].as_str().expect("settled invoice has proof")).unwrap();
        assert_eq!(preimage.len(), 32);
        let invoice: Bolt11Invoice = pr.parse().unwrap();
        assert_eq!(&sha256::Hash::hash(&preimage), invoice.payment_hash());
        assert_eq!(
            response,
            &json!({"status": "OK", "settled": true, "pr": pr, "preimage": hex::encode(preimage)})
        );
    }

    #[tokio::test]
    async fn well_known_advertises_a_reachable_pay_endpoint() {
        let app = app().await;
        let token = app.admin_token().await;
        let (_wallet, addr) = register_address(app, token, "lnurl-meta", None).await;

        let res = app
            .api()
            .get(&format!("/.well-known/lnurlp/{}", addr.username), Auth::None)
            .await;
        assert_status(&res, StatusCode::OK);
        let pay = res.parse::<LnURLPayRequest>();
        assert_eq!(pay.tag, "payRequest");
        assert!(
            pay.min_sendable >= 1 && pay.min_sendable <= pay.max_sendable,
            "sendable range is sane: {}..={}",
            pay.min_sendable,
            pay.max_sendable
        );
        assert!(
            pay.callback.starts_with(&app.base_url),
            "the advertised callback targets this server: {} (base {})",
            pay.callback,
            app.base_url
        );
        assert!(
            pay.callback.contains(&addr.username),
            "the callback identifies the user"
        );
        assert!(
            !pay.metadata.is_empty(),
            "metadata is served for signature verification"
        );
    }

    #[tokio::test]
    async fn the_advertised_callback_issues_a_bolt11() {
        let app = app().await;
        let token = app.admin_token().await;
        let (_wallet, addr) = register_address(app, token, "lnurl-cb", None).await;

        let pay = app
            .api()
            .get(&format!("/.well-known/lnurlp/{}", addr.username), Auth::None)
            .await
            .parse::<LnURLPayRequest>();
        let cb = follow_callback(&pay.callback, 100_000_000).await;
        assert!(
            cb.pr.to_lowercase().starts_with("lnbcrt"),
            "a regtest bolt11 invoice was issued: {}",
            cb.pr
        );
        assert!(cb.success_action.is_some());
    }

    #[tokio::test]
    async fn callback_rejects_amounts_outside_the_advertised_range() {
        let app = app().await;
        let token = app.admin_token().await;
        let (wallet, addr) = register_address(app, token, "lnurl-range-reject", None).await;
        let pay = app
            .api()
            .get(&format!("/.well-known/lnurlp/{}", addr.username), Auth::None)
            .await
            .parse::<LnURLPayRequest>();
        let balance_before = app.wallet_balance(token, wallet.id).await;

        for amount in [
            0,
            pay.min_sendable - 1,
            pay.max_sendable + 1,
            i64::MAX as u64 + 1,
            u64::MAX,
        ] {
            let response = reqwest::get(format!("{}?amount={amount}", pay.callback))
                .await
                .expect("reach the LNURL callback");
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let body: Value = response.json().await.expect("validation error response");
            assert!(body["reason"].is_string());
        }

        let invoices = app
            .api()
            .get(&format!("/v1/invoices?wallet_id={}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&invoices, StatusCode::OK);
        assert!(invoices.parse::<Vec<Invoice>>().is_empty());
        let balance_after = app.wallet_balance(token, wallet.id).await;
        assert_eq!(balance_before.available_msat, balance_after.available_msat);
        assert_eq!(balance_before.reserved_msat, balance_after.reserved_msat);
    }

    #[tokio::test]
    async fn callback_accepts_both_advertised_amount_boundaries() {
        let app = app().await;
        let token = app.admin_token().await;
        let (wallet, addr) = register_address(app, token, "lnurl-range-accept", None).await;
        let pay = app
            .api()
            .get(&format!("/.well-known/lnurlp/{}", addr.username), Auth::None)
            .await
            .parse::<LnURLPayRequest>();

        for amount in [pay.min_sendable, pay.max_sendable] {
            let callback = follow_callback(&pay.callback, amount).await;
            let invoice: Bolt11Invoice = callback.pr.parse().expect("valid BOLT11 invoice");
            assert_eq!(invoice.amount_milli_satoshis(), Some(amount));
        }

        let invoices = app
            .api()
            .get(&format!("/v1/invoices?wallet_id={}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&invoices, StatusCode::OK);
        let invoices = invoices.parse::<Vec<Invoice>>();
        assert_eq!(invoices.len(), 2);
        assert!(invoices
            .iter()
            .any(|invoice| invoice.amount_msat == Some(pay.min_sendable)));
        assert!(invoices
            .iter()
            .any(|invoice| invoice.amount_msat == Some(pay.max_sendable)));
    }

    #[tokio::test]
    async fn an_external_payer_settles_via_the_address() {
        let app = app().await;
        let token = app.admin_token().await;
        let (wallet, addr) = register_address(app, token, "lnurl-pay", None).await;

        let amount_msat = 100_000_000u64;
        let pay = app
            .api()
            .get(&format!("/.well-known/lnurlp/{}", addr.username), Auth::None)
            .await
            .parse::<LnURLPayRequest>();
        // Resolve and pay strictly through the advertised callback.
        let cb = follow_callback(&pay.callback, amount_msat).await;
        let verify_url = cb.verify.as_deref().expect("callback advertises LUD-21");
        let another = follow_callback(&pay.callback, amount_msat).await;
        let another_url = another.verify.as_deref().expect("each invoice has a verification URL");
        assert_ne!(verify_url, another_url);
        assert_eq!(
            verify(verify_url).await,
            json!({"status": "OK", "settled": false, "pr": cb.pr, "preimage": null})
        );
        let invoices = app
            .api()
            .get(&format!("/v1/invoices?wallet_id={}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&invoices, StatusCode::OK);
        for invoice in invoices.body.as_array().unwrap() {
            assert!(
                invoice.get("lnurl").is_none(),
                "private verification data is omitted from invoice JSON"
            );
        }
        Counterparty::for_provider(&app.provider).pay(&cb.pr);

        wait_until(
            Duration::from_secs(45),
            "wallet credited via its lightning address",
            || async { app.wallet_balance(token, wallet.id).await.available_msat >= amount_msat as i64 },
        )
        .await;

        assert_payment_proof(&verify(verify_url).await, &cb.pr);
        assert_eq!(
            verify(another_url).await,
            json!({"status": "OK", "settled": false, "pr": another.pr, "preimage": null})
        );
    }

    #[tokio::test]
    async fn verifies_an_internal_payment_after_restart() {
        let app = TestApp::isolated("lnurl-verify-internal", &[]).await;
        let token = app.admin_token().await;
        let payer = app.create_wallet(token, "lnurl-verify-payer").await;
        app.fund_onchain(token, payer.id, 100_000).await;
        let (_wallet, addr) = register_address(&app, token, "lnurl-verify-payee", None).await;
        let pay = app
            .api()
            .get(&format!("/.well-known/lnurlp/{}", addr.username), Auth::None)
            .await
            .parse::<LnURLPayRequest>();
        let cb = follow_callback(&pay.callback, 10_000_000).await;
        let verify_url = reqwest::Url::parse(cb.verify.as_deref().unwrap()).unwrap();
        let result = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                SendPaymentRequest {
                    wallet_id: Some(payer.id),
                    input: cb.pr.clone(),
                    amount_msat: None,
                    comment: None,
                },
            )
            .await;
        assert_status(&result, StatusCode::OK);
        assert_eq!(result.parse::<Payment>().ledger, Ledger::Internal);
        assert_payment_proof(&verify(verify_url.as_str()).await, &cb.pr);

        let restarted = app.restart("lnurl-verify-restarted", &[]).await;
        let url = format!("{}{}", restarted.base_url, verify_url.path());
        assert_payment_proof(&verify(&url).await, &cb.pr);
    }

    #[tokio::test]
    async fn verification_rejects_invalid_tokens_and_ordinary_invoice_ids() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-verify-private").await;
        let result = app
            .api()
            .post(
                "/v1/invoices",
                Auth::Bearer(token),
                NewInvoiceRequest {
                    wallet_id: Some(wallet.id),
                    amount_msat: 10_000,
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_status(&result, StatusCode::OK);
        let id = result.parse::<Invoice>().id;

        for token in [
            "not-a-uuid".to_string(),
            uuid::Uuid::new_v4().to_string(),
            id.to_string(),
        ] {
            let url = format!("{}/lnurlp/{token}/verify", app.base_url);
            assert_eq!(verify(&url).await, json!({"status": "ERROR", "reason": "Not found"}));
        }
    }

    #[tokio::test]
    async fn an_inactive_address_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let (_wallet, addr) = register_address(app, token, "lnurl-inactive", None).await;

        // Deactivate the address; an inactive address cannot receive.
        let updated = app
            .api()
            .put(
                &format!("/v1/lightning-addresses/{}", addr.id),
                Auth::Bearer(token),
                UpdateLnAddressRequest {
                    username: None,
                    active: Some(false),
                    allows_nostr: None,
                    nostr_pubkey: None,
                },
            )
            .await;
        assert_status(&updated, StatusCode::OK);
        assert!(!updated.parse::<LnAddress>().active);

        // The public endpoint no longer resolves it.
        let res = app
            .api()
            .get(&format!("/.well-known/lnurlp/{}", addr.username), Auth::None)
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_username_is_not_found() {
        let app = app().await;
        let res = app
            .api()
            .get(&format!("/.well-known/lnurlp/{}", unique("nobody")), Auth::None)
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }
}

mod nostr {
    use super::*;

    // The secp256k1 generator x-coordinate: a valid BIP-340 x-only key, used as a
    // deterministic Nostr pubkey for the NIP-05 mapping.
    const NOSTR_PUBKEY: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    #[tokio::test]
    async fn nip05_returns_the_registered_pubkey() {
        let app = app().await;
        let token = app.admin_token().await;
        let (_wallet, addr) = register_address(app, token, "nip05", Some(NOSTR_PUBKEY)).await;
        assert!(addr.allows_nostr, "registering with a pubkey enables nostr");

        let res = app
            .api()
            .get(&format!("/.well-known/nostr.json?name={}", addr.username), Auth::None)
            .await;
        assert_status(&res, StatusCode::OK);
        let names = res.parse::<NostrNIP05Response>().names;
        assert_eq!(
            names.get(&addr.username).map(String::as_str),
            Some(NOSTR_PUBKEY),
            "NIP-05 maps the username to its registered pubkey"
        );
    }

    #[tokio::test]
    async fn an_address_without_nostr_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let (_wallet, addr) = register_address(app, token, "nip05-off", None).await;
        assert!(!addr.allows_nostr);

        let res = app
            .api()
            .get(&format!("/.well-known/nostr.json?name={}", addr.username), Auth::None)
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_name_is_not_found() {
        let app = app().await;
        let res = app
            .api()
            .get(
                &format!("/.well-known/nostr.json?name={}", unique("nobody")),
                Auth::None,
            )
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }
}
