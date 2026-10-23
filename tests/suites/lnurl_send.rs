//! Paying *out* to an external LNURL-pay service (`/v1/payments`). The external
//! service is a wiremock mock that SwissKnife resolves over HTTP: it GETs the
//! pay-request metadata, follows the advertised callback to obtain a bolt11,
//! validates it, then pays it. The settled happy path runs over the real
//! LND<->CLN channel (so it joins the provider matrix); the validation/error
//! paths short-circuit before any reserve, so they need no funds or LN routing.
//!
//! Counterpart to `well_known.rs`, which covers serving *our* LNURL endpoints.
//! The `user@host` lightning-address form is not exercised here: the `lnurl`
//! crate forces `https` for it, unreachable for a `127.0.0.1` mock — the raw
//! `http://` URL input drives the identical outbound code path.

use reqwest::StatusCode;
use serde_json::json;

use swissknife_types::{Ledger, Payment, PaymentFeeEstimate, PaymentStatus, SendPaymentRequest};

use crate::common::counterparty::Counterparty;
use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth, MockLnurl};

/// Build a pay-to-LNURL request that targets the mock as a raw `http://` URL.
fn pay(wallet_id: uuid::Uuid, input: String, amount_msat: u64, comment: Option<&str>) -> SendPaymentRequest {
    SendPaymentRequest {
        wallet_id: Some(wallet_id),
        input,
        amount_msat: Some(amount_msat),
        comment: comment.map(str::to_string),
    }
}

mod pay {
    use super::*;

    /// Full outbound flow over the real channel: resolve the mock pay-request,
    /// follow its callback to a counterparty-issued bolt11, pay it, settle, and
    /// debit the wallet — asserting the mock saw the expected callback request.
    #[tokio::test]
    async fn settles_decrypts_and_persists_the_success_action() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-send").await;

        app.fund_onchain(token, wallet.id, 1_000_000).await;
        let before = app.wallet_balance(token, wallet.id).await.available_msat;

        let amount_msat = 100_000_000u64;
        // The invoice the mock hands back must be payable on the regtest network,
        // so it is issued by the counterparty for exactly the requested amount.
        let first_half = uuid::Uuid::new_v4();
        let second_half = uuid::Uuid::new_v4();
        let mut payment_preimage = [0u8; 32];
        payment_preimage[..16].copy_from_slice(first_half.as_bytes());
        payment_preimage[16..].copy_from_slice(second_half.as_bytes());
        let bolt11 = Counterparty::for_provider(&app.provider).invoice_with_preimage(
            amount_msat,
            &unique("lnurl-cp"),
            &payment_preimage,
        );

        let user = unique("payee");
        let mock = MockLnurl::start().await;
        mock.mount_pay_request(&user, 1_000, 250_000_000_000, 255).await;
        mock.mount_callback_invoice_with_aes(&bolt11, "LNURL receipt", "Thanks for the sats!", &payment_preimage)
            .await;

        let quote = app
            .api()
            .post(
                "/v1/payments/fee-estimate",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), amount_msat, None),
            )
            .await;
        assert_status(&quote, StatusCode::OK);
        let quote = quote.parse::<PaymentFeeEstimate>();
        assert_eq!(quote.ledger, Ledger::Lightning);
        assert_eq!(quote.amount_msat, amount_msat);
        let estimated_fee_msat = quote
            .estimated_fee_msat
            .expect("the real Lightning provider returns an LNURL route estimate");
        assert!(estimated_fee_msat <= quote.maximum_fee_msat);

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), amount_msat, None),
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let payment = res.parse::<Payment>();
        assert_eq!(
            payment.status,
            PaymentStatus::Settled,
            "payment not settled: {payment:?}"
        );
        assert_eq!(payment.ledger, Ledger::Lightning);
        assert_eq!(payment.amount_msat, amount_msat);

        // The callback action is persisted before paying. Whether the synchronous
        // request or the provider listener wins settlement, the preimage projects it
        // onto the public payment deterministically.
        let success_action = payment
            .lightning
            .as_ref()
            .and_then(|ln| ln.success_action.as_ref())
            .expect("LNURL success action present on the settled payment");
        assert_eq!(success_action.tag, "message");
        assert_eq!(success_action.message.as_deref(), Some("Thanks for the sats!"));
        assert_eq!(success_action.description.as_deref(), Some("LNURL receipt"));

        // Re-read through the public API rather than trusting the POST response:
        // this proves the action survived the database serialization boundary.
        let persisted = app
            .api()
            .get(&format!("/v1/payments/{}", payment.id), Auth::Bearer(token))
            .await;
        assert_status(&persisted, StatusCode::OK);
        let persisted = persisted.parse::<Payment>();
        let persisted_action = persisted
            .lightning
            .as_ref()
            .and_then(|ln| ln.success_action.as_ref())
            .expect("LNURL success action persisted on the settled payment");
        assert_eq!(persisted_action.tag, "message");
        assert_eq!(persisted_action.message.as_deref(), Some("Thanks for the sats!"));
        assert_eq!(persisted_action.description.as_deref(), Some("LNURL receipt"));

        // The wallet is debited by the amount plus the routing fee.
        let fee = payment.fee_msat.unwrap_or_default() as i64;
        assert!(
            fee as u64 <= quote.maximum_fee_msat,
            "settled fee stays within the quoted cap"
        );
        let after = app.wallet_balance(token, wallet.id).await.available_msat;
        assert_eq!(
            after,
            before - amount_msat as i64 - fee,
            "wallet debited by amount + fee (before={before}, fee={fee})"
        );

        // The mock must have been asked for an invoice for exactly this amount:
        // proof SwissKnife sent the expected outbound request.
        let callbacks = mock.callback_requests().await;
        assert_eq!(callbacks.len(), 2, "the quote and payment each requested an invoice");
        let amount_param = callbacks[0]
            .url
            .query_pairs()
            .find(|(k, _)| k == "amount")
            .map(|(_, v)| v.into_owned());
        assert_eq!(amount_param.as_deref(), Some(amount_msat.to_string().as_str()));
    }

    /// The comment is forwarded to the callback as a query parameter. Asserted
    /// without settling: the callback declines, but the request it received
    /// still proves SwissKnife sent the comment (and the amount).
    #[tokio::test]
    async fn forwards_the_comment_to_the_callback() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-comment").await;

        let amount_msat = 100_000u64;
        let comment = "thanks for the coffee";

        let user = unique("payee");
        let mock = MockLnurl::start().await;
        mock.mount_pay_request(&user, 1_000, 1_000_000_000, 255).await;
        mock.mount_callback_error("not today").await;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), amount_msat, Some(comment)),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);

        let callbacks = mock.callback_requests().await;
        assert_eq!(callbacks.len(), 1, "the callback was hit once");
        let pairs: Vec<(String, String)> = callbacks[0].url.query_pairs().into_owned().collect();
        assert!(
            pairs
                .iter()
                .any(|(k, v)| k == "amount" && v == &amount_msat.to_string()),
            "callback received the amount: {pairs:?}"
        );
        assert!(
            pairs.iter().any(|(k, v)| k == "comment" && v == comment),
            "callback received the comment: {pairs:?}"
        );
    }
}

mod validation {
    use super::*;
    use std::time::Duration;
    use wiremock::ResponseTemplate;

    #[tokio::test]
    async fn slow_metadata_is_bounded() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-slow-metadata").await;
        let mock = MockLnurl::start().await;
        let user = unique("payee");
        mock.mount_response(
            &format!("/.well-known/lnurlp/{user}"),
            ResponseTemplate::new(200)
                .set_body_json(json!({"tag": "notAPayRequest"}))
                .set_delay(Duration::from_secs(20)),
        )
        .await;

        let res = tokio::time::timeout(
            Duration::from_secs(15),
            app.api().post(
                "/v1/payments/fee-estimate",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            ),
        )
        .await
        .expect("metadata fetch must stop at its own 10-second deadline");
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(mock.callback_requests().await.is_empty());
    }

    #[tokio::test]
    async fn slow_callback_is_bounded() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-slow-callback").await;
        let mock = MockLnurl::start().await;
        let user = unique("payee");
        mock.mount_pay_request(&user, 1_000, 1_000_000, 0).await;
        mock.mount_response(
            "/lnurl/callback",
            ResponseTemplate::new(200)
                .set_body_json(json!({"status": "ERROR", "reason": "too late"}))
                .set_delay(Duration::from_secs(20)),
        )
        .await;

        let res = tokio::time::timeout(
            Duration::from_secs(15),
            app.api().post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            ),
        )
        .await
        .expect("callback fetch must stop at its own 10-second deadline");
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!res.body["reason"].as_str().unwrap_or_default().contains("too late"));
        assert_eq!(mock.callback_requests().await.len(), 1);
        let balance = app.wallet_balance(token, wallet.id).await;
        assert_eq!(balance.available_msat, 0);
        assert_eq!(balance.reserved_msat, 0);
    }

    #[tokio::test]
    async fn metadata_at_the_size_limit_reaches_the_callback() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-boundary-metadata").await;
        let mock = MockLnurl::start().await;
        let user = unique("payee");
        let mut metadata = json!({
            "tag": "payRequest",
            "callback": format!("{}/lnurl/callback", mock.uri()),
            "minSendable": 1_000,
            "maxSendable": 1_000_000,
            "metadata": json!([
                ["text/plain", "Large image metadata"],
                ["image/png;base64", "A".repeat(136_536)],
            ]).to_string(),
            "padding": "",
        });
        let padding = 256 * 1024 - serde_json::to_vec(&metadata).unwrap().len();
        metadata["padding"] = json!(" ".repeat(padding));
        assert_eq!(serde_json::to_vec(&metadata).unwrap().len(), 256 * 1024);
        mock.mount_metadata(&user, metadata).await;
        mock.mount_callback_error("size boundary accepted").await;

        let res = app
            .api()
            .post(
                "/v1/payments/fee-estimate",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(res.body["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("size boundary accepted"));
        assert_eq!(mock.callback_requests().await.len(), 1);
    }

    #[tokio::test]
    async fn callback_at_the_size_limit_preserves_the_remote_reason() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-boundary-callback").await;
        let mock = MockLnurl::start().await;
        let user = unique("payee");
        mock.mount_pay_request(&user, 1_000, 1_000_000, 0).await;
        let mut body = json!({"status": "ERROR", "reason": "size boundary accepted", "padding": ""});
        let padding = 256 * 1024 - serde_json::to_vec(&body).unwrap().len();
        body["padding"] = json!(" ".repeat(padding));
        assert_eq!(serde_json::to_vec(&body).unwrap().len(), 256 * 1024);
        mock.mount_response("/lnurl/callback", ResponseTemplate::new(200).set_body_json(body))
            .await;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(res.body["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("size boundary accepted"));
    }

    #[tokio::test]
    async fn chunked_metadata_without_content_length_is_limited() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-chunked-metadata").await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let router = axum::Router::new().route(
            "/",
            axum::routing::get(|| async {
                axum::body::Body::from_stream(futures_util::stream::iter(
                    (0..17).map(|_| Ok::<_, std::io::Error>(vec![b'a'; 16 * 1024])),
                ))
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let probe = reqwest::get(&url).await.unwrap();
        assert_eq!(probe.content_length(), None, "fixture must not advertise a body length");
        drop(probe);

        let res = app
            .api()
            .post(
                "/v1/payments/fee-estimate",
                Auth::Bearer(token),
                pay(wallet.id, url, 100_000, None),
            )
            .await;
        server.abort();
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(res.body["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("LNURL response exceeds 262144 bytes"));
    }

    #[tokio::test]
    async fn a_stalled_callback_body_is_bounded_after_headers() {
        use futures_util::{stream, StreamExt};

        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-stalled-body").await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let router = axum::Router::new().route(
            "/",
            axum::routing::get(|| async {
                let chunks =
                    stream::iter([Ok::<_, std::io::Error>("{\"status\":\"ERROR\",")]).chain(stream::once(async {
                        tokio::time::sleep(Duration::from_secs(20)).await;
                        Ok("\"reason\":\"too late\"}")
                    }));
                axum::body::Body::from_stream(chunks)
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let probe = tokio::time::timeout(Duration::from_secs(2), reqwest::get(&url))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(probe.content_length(), None);
        drop(probe);
        let mock = MockLnurl::start().await;
        let user = unique("payee");
        mock.mount_metadata(
            &user,
            json!({
                "tag": "payRequest", "callback": url,
                "minSendable": 1_000, "maxSendable": 1_000_000, "metadata": "[]",
            }),
        )
        .await;

        let res = tokio::time::timeout(
            Duration::from_secs(15),
            app.api().post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            ),
        )
        .await
        .expect("the deadline must include body consumption, not only response headers");
        server.abort();
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!res.body["reason"].as_str().unwrap_or_default().contains("too late"));
    }

    #[tokio::test]
    async fn oversized_metadata_is_rejected_before_callback() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-large-metadata").await;
        let mock = MockLnurl::start().await;
        let user = unique("payee");
        mock.mount_metadata(
            &user,
            json!({
                "tag": "payRequest",
                "callback": format!("{}/lnurl/callback", mock.uri()),
                "minSendable": 1_000,
                "maxSendable": 1_000_000,
                "metadata": "[]",
                "padding": "a".repeat(256 * 1024 + 1),
            }),
        )
        .await;
        mock.mount_callback_error("metadata was accepted").await;

        for endpoint in ["/v1/payments/fee-estimate", "/v1/payments"] {
            let res = app
                .api()
                .post(
                    endpoint,
                    Auth::Bearer(token),
                    pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
                )
                .await;
            assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
            assert!(res.body["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("LNURL response exceeds 262144 bytes"));
        }
        assert!(mock.callback_requests().await.is_empty());
        let balance = app.wallet_balance(token, wallet.id).await;
        assert_eq!(balance.available_msat, 0);
        assert_eq!(balance.reserved_msat, 0);
        let payments = app
            .api()
            .get(&format!("/v1/payments?wallet_id={}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&payments, StatusCode::OK);
        assert!(payments.parse::<Vec<Payment>>().is_empty());
    }

    #[tokio::test]
    async fn oversized_callback_is_rejected_before_payment() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-large-callback").await;
        let mock = MockLnurl::start().await;
        let user = unique("payee");
        mock.mount_pay_request(&user, 1_000, 1_000_000, 0).await;
        mock.mount_callback_error(&"a".repeat(256 * 1024 + 1)).await;

        for endpoint in ["/v1/payments/fee-estimate", "/v1/payments"] {
            let res = app
                .api()
                .post(
                    endpoint,
                    Auth::Bearer(token),
                    pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
                )
                .await;
            assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
            assert!(res.body["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("LNURL response exceeds 262144 bytes"));
        }
        assert_eq!(mock.callback_requests().await.len(), 2);
        let balance = app.wallet_balance(token, wallet.id).await;
        assert_eq!(balance.available_msat, 0);
        assert_eq!(balance.reserved_msat, 0);
    }

    /// An `ERROR` response from the external callback is surfaced as a 422 with
    /// the remote reason preserved.
    #[tokio::test]
    async fn a_callback_error_is_surfaced() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-cberr").await;

        let user = unique("payee");
        let mock = MockLnurl::start().await;
        mock.mount_pay_request(&user, 1_000, 1_000_000_000, 255).await;
        mock.mount_callback_error("recipient is offline").await;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
        let reason = res.body["reason"].as_str().unwrap_or_default();
        assert!(
            reason.contains("recipient is offline"),
            "remote reason preserved: {reason}"
        );
    }

    /// The callback returns an invoice for a different amount than requested:
    /// SwissKnife must reject it rather than overpay/underpay.
    #[tokio::test]
    async fn a_callback_invoice_with_a_mismatched_amount_is_rejected() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-mismatch").await;

        // Requested 100_000 msat, but the callback hands back a 250_000 msat invoice.
        let bolt11 = Counterparty::for_provider(&app.provider).invoice(250_000, &unique("lnurl-mm"));

        let user = unique("payee");
        let mock = MockLnurl::start().await;
        mock.mount_pay_request(&user, 1_000, 1_000_000_000, 255).await;
        mock.mount_callback_invoice(&bolt11).await;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// The callback returns something that is not a bolt11 invoice.
    #[tokio::test]
    async fn a_callback_with_an_unparseable_invoice_is_rejected() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-badpr").await;

        let user = unique("payee");
        let mock = MockLnurl::start().await;
        mock.mount_pay_request(&user, 1_000, 1_000_000_000, 255).await;
        mock.mount_callback_invoice("not-a-bolt11-invoice").await;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// Metadata that is not a valid LNURL pay-request (unknown `tag`) is rejected
    /// during resolution.
    #[tokio::test]
    async fn non_lnurl_pay_metadata_is_rejected() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-badmeta").await;

        let user = unique("payee");
        let mock = MockLnurl::start().await;
        mock.mount_metadata(&user, json!({ "tag": "notAPayRequest" })).await;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// An endpoint that does not resolve (nothing mounted -> 404) is rejected.
    #[tokio::test]
    async fn an_unresolvable_endpoint_is_rejected() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-404").await;

        let mock = MockLnurl::start().await;
        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&unique("ghost")), 100_000, None),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// An amount outside the advertised sendable range is rejected locally,
    /// before the callback is ever contacted.
    #[tokio::test]
    async fn an_amount_above_max_sendable_is_rejected_without_calling_back() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-range").await;

        let user = unique("payee");
        let mock = MockLnurl::start().await;
        // Advertise a low ceiling, then ask to send above it.
        mock.mount_pay_request(&user, 1_000, 50_000, 255).await;
        mock.mount_callback_invoice("unused").await;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                pay(wallet.id, mock.lnurlp_url(&user), 100_000, None),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);

        // SwissKnife enforces the range before fetching an invoice.
        assert!(
            mock.callback_requests().await.is_empty(),
            "callback must not be contacted for an out-of-range amount"
        );
    }
}
