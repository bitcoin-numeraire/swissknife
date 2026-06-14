//! Public, unauthenticated endpoints an external wallet hits to reach a
//! SwissKnife user: LNURL-pay (LUD-06) and Nostr NIP-05. These serve OUR
//! endpoints — no outbound calls — so no mock server is needed, and the LNURL
//! pay flow settles over the real LND<->CLN channel. Paying *out* to someone
//! else's address/Nostr (which needs a mock server) is covered separately.

use std::time::Duration;

use reqwest::StatusCode;

use swissknife_types::{LnURLPayRequest, LnUrlCallback, NostrNIP05Response, Wallet};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::unique;
use crate::common::wait::wait_until;
use crate::common::{app, assert_error, assert_status, Auth, TestApp};

/// Register an LN address (and optionally a Nostr key) under a fresh wallet,
/// returning the wallet and the username that keys its public endpoints.
async fn register_address(app: &TestApp, token: &str, label: &str, nostr_pubkey: Option<&str>) -> (Wallet, String) {
    let wallet = app.create_wallet(token, label).await;
    let username = unique(label);
    let body = serde_json::json!({
        "wallet_id": wallet.id,
        "username": username,
        "allows_nostr": nostr_pubkey.is_some(),
        "nostr_pubkey": nostr_pubkey,
    });
    let res = app
        .api()
        .post("/v1/lightning-addresses", Auth::Bearer(token), body)
        .await;
    assert_status(&res, StatusCode::OK);
    (wallet, username)
}

mod lnurl {
    use super::*;

    #[tokio::test]
    async fn well_known_describes_the_pay_endpoint() {
        let app = app().await;
        let token = app.admin_token().await;
        let (_wallet, username) = register_address(app, token, "lnurl-meta", None).await;

        let res = app
            .api()
            .get(&format!("/.well-known/lnurlp/{username}"), Auth::None)
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
            pay.callback.contains(&username),
            "the callback URL targets this user: {}",
            pay.callback
        );
        assert!(
            !pay.metadata.is_empty(),
            "metadata is served for signature verification"
        );
    }

    #[tokio::test]
    async fn callback_issues_a_bolt11_for_the_requested_amount() {
        let app = app().await;
        let token = app.admin_token().await;
        let (_wallet, username) = register_address(app, token, "lnurl-cb", None).await;

        let res = app
            .api()
            .get(&format!("/lnurlp/{username}/callback?amount=100000000"), Auth::None)
            .await;
        assert_status(&res, StatusCode::OK);
        let cb = res.parse::<LnUrlCallback>();
        assert!(
            cb.pr.to_lowercase().starts_with("lnbcrt"),
            "a regtest bolt11 invoice was issued: {}",
            cb.pr
        );
    }

    #[tokio::test]
    async fn an_external_payer_settles_via_the_address() {
        let app = app().await;
        let token = app.admin_token().await;
        let (wallet, username) = register_address(app, token, "lnurl-pay", None).await;

        let amount_msat = 100_000_000u64;
        let cb = app
            .api()
            .get(&format!("/lnurlp/{username}/callback?amount={amount_msat}"), Auth::None)
            .await
            .parse::<LnUrlCallback>();

        // An external wallet pays the invoice the callback issued over the channel.
        Counterparty::for_provider(&app.provider).pay(&cb.pr);

        wait_until(
            Duration::from_secs(45),
            "wallet credited via its lightning address",
            || async { app.wallet_balance(token, wallet.id).await.available_msat >= amount_msat as i64 },
        )
        .await;
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
        let (_wallet, username) = register_address(app, token, "nip05", Some(NOSTR_PUBKEY)).await;

        let res = app
            .api()
            .get(&format!("/.well-known/nostr.json?name={username}"), Auth::None)
            .await;
        assert_status(&res, StatusCode::OK);
        let names = res.parse::<NostrNIP05Response>().names;
        assert_eq!(
            names.get(&username).map(String::as_str),
            Some(NOSTR_PUBKEY),
            "NIP-05 maps the username to its registered pubkey"
        );
    }

    #[tokio::test]
    async fn an_address_without_nostr_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let (_wallet, username) = register_address(app, token, "nip05-off", None).await;

        let res = app
            .api()
            .get(&format!("/.well-known/nostr.json?name={username}"), Auth::None)
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
