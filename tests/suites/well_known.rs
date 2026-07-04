//! Public, unauthenticated endpoints an external wallet hits to reach a
//! SwissKnife user: LNURL-pay (LUD-06) and Nostr NIP-05. These serve OUR
//! endpoints — no outbound calls — so no mock server is needed, and the LNURL
//! pay flow settles over the real LND<->CLN channel. Paying *out* to someone
//! else's address/Nostr (which needs a mock server) is covered separately.

use std::time::Duration;

use reqwest::StatusCode;

use swissknife_types::{LnAddress, LnURLPayRequest, LnUrlCallback, NostrNIP05Response, UpdateLnAddressRequest, Wallet};

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
        res.json::<LnUrlCallback>()
            .await
            .expect("callback returns an LnUrlCallback")
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
        Counterparty::for_provider(&app.provider).pay(&cb.pr);

        wait_until(
            Duration::from_secs(45),
            "wallet credited via its lightning address",
            || async { app.wallet_balance(token, wallet.id).await.available_msat >= amount_msat as i64 },
        )
        .await;
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
