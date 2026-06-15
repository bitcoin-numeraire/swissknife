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

use swissknife_types::{Ledger, Payment, PaymentStatus, SendPaymentRequest};

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
    async fn settles_against_an_external_lnurl_service_and_debits_the_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnurl-send").await;

        app.fund_onchain(token, wallet.id, 1_000_000).await;
        let before = app.wallet_balance(token, wallet.id).await.available_msat;

        let amount_msat = 100_000_000u64;
        // The invoice the mock hands back must be payable on the regtest network,
        // so it is issued by the counterparty for exactly the requested amount.
        let bolt11 = Counterparty::for_provider(&app.provider).invoice(amount_msat, &unique("lnurl-cp"));

        let user = unique("payee");
        let mock = MockLnurl::start().await;
        mock.mount_pay_request(&user, 1_000, 250_000_000_000, 255).await;
        mock.mount_callback_invoice_with_message(&bolt11, "Thanks for the sats!")
            .await;

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

        // The success action returned by the callback is mapped onto the payment.
        let success_action = payment
            .lightning
            .as_ref()
            .and_then(|ln| ln.success_action.as_ref())
            .expect("a success action is recorded");
        assert_eq!(success_action.message.as_deref(), Some("Thanks for the sats!"));

        // The wallet is debited by the amount plus the routing fee.
        let fee = payment.fee_msat.unwrap_or_default() as i64;
        let after = app.wallet_balance(token, wallet.id).await.available_msat;
        assert_eq!(
            after,
            before - amount_msat as i64 - fee,
            "wallet debited by amount + fee (before={before}, fee={fee})"
        );

        // The mock must have been asked for an invoice for exactly this amount:
        // proof SwissKnife sent the expected outbound request.
        let callbacks = mock.callback_requests().await;
        assert_eq!(callbacks.len(), 1, "the advertised callback was hit exactly once");
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
