//! A wiremock-backed mock of an *external* LNURL-pay service, used to test
//! SwissKnife's outbound pay path: resolve a pay-request (LUD-06), follow the
//! advertised callback, validate the returned invoice, then pay it.
//!
//! Well-formed responses are built from the `lnurl` crate's own wire types
//! (`PayResponse`, `LnURLPayInvoice`) — the same types SwissKnife parses, so the
//! mock round-trips by construction. Raw JSON is used only where the case is
//! deliberately not expressible via those types: a success action (the crate's
//! field is private), the `{status, reason}` error contract, and malformed
//! bodies (the whole point of those tests).
//!
//! SwissKnife is pointed at the mock with a raw `http://` URL payment input; the
//! `user@host` lightning-address form is not used here because the `lnurl` crate
//! forces `https` for it (a `127.0.0.1` mock is only reachable over `http`). The
//! raw-URL path drives the identical resolve→callback→validate→pay logic. Each
//! test gets its own server (own port), so mounts never collide.

use lnurl::pay::{LnURLPayInvoice, PayResponse};
use lnurl::Tag;
use serde::Serialize;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Path the mock advertises (and serves) as the pay-request callback.
const CALLBACK_PATH: &str = "/lnurl/callback";

pub struct MockLnurl {
    server: MockServer,
}

impl MockLnurl {
    pub async fn start() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }

    /// The mock's base URL, e.g. `http://127.0.0.1:<port>`.
    pub fn uri(&self) -> String {
        self.server.uri()
    }

    /// The pay-request URL to hand SwissKnife as a payment input. SwissKnife
    /// `GET`s this, expecting LUD-06 pay-request metadata.
    pub fn lnurlp_url(&self, user: &str) -> String {
        format!("{}/.well-known/lnurlp/{user}", self.server.uri())
    }

    fn callback_url(&self) -> String {
        format!("{}{CALLBACK_PATH}", self.server.uri())
    }

    /// Mount the LUD-06 pay-request metadata at `/.well-known/lnurlp/{user}`,
    /// advertising this mock's own callback. Amounts are in millisatoshis.
    pub async fn mount_pay_request(&self, user: &str, min_sendable: u64, max_sendable: u64, comment_allowed: u16) {
        let pay = PayResponse {
            callback: self.callback_url(),
            max_sendable,
            min_sendable,
            tag: Tag::PayRequest,
            metadata: "[[\"text/plain\",\"itest lnurl pay\"]]".to_string(),
            comment_allowed: Some(u32::from(comment_allowed)),
            allows_nostr: None,
            nostr_pubkey: None,
        };
        self.mount_get(format!("/.well-known/lnurlp/{user}"), pay).await;
    }

    /// Mount an arbitrary JSON body at the pay-request path, for malformed or
    /// non-pay-request (e.g. unknown `tag`) metadata cases the typed
    /// `PayResponse` cannot express.
    pub async fn mount_metadata(&self, user: &str, body: Value) {
        self.mount_get(format!("/.well-known/lnurlp/{user}"), body).await;
    }

    /// Mount the callback so it returns `bolt11` as the invoice to pay. `bolt11`
    /// need not be valid — the unparseable-invoice case passes junk here.
    pub async fn mount_callback_invoice(&self, bolt11: &str) {
        self.mount_get(CALLBACK_PATH.to_string(), LnURLPayInvoice::new(bolt11.to_string()))
            .await;
    }

    /// Mount the callback so it returns `bolt11` plus a `message` success action.
    /// Raw JSON: `LnURLPayInvoice`'s success-action field is private with no setter.
    pub async fn mount_callback_invoice_with_message(&self, bolt11: &str, message: &str) {
        self.mount_get(
            CALLBACK_PATH.to_string(),
            json!({ "pr": bolt11, "successAction": { "tag": "message", "message": message } }),
        )
        .await;
    }

    /// Mount the callback so it returns the LNURL error contract `{status, reason}`.
    pub async fn mount_callback_error(&self, reason: &str) {
        self.mount_get(
            CALLBACK_PATH.to_string(),
            json!({ "status": "ERROR", "reason": reason }),
        )
        .await;
    }

    /// Mount a `200 GET` returning `body` at `at`.
    async fn mount_get(&self, at: String, body: impl Serialize) {
        Mock::given(method("GET"))
            .and(path(at))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&self.server)
            .await;
    }

    /// All callback requests SwissKnife actually made (to assert the outbound
    /// contract: the `amount`/`comment` query it sent).
    pub async fn callback_requests(&self) -> Vec<Request> {
        self.server
            .received_requests()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|req| req.url.path() == CALLBACK_PATH)
            .collect()
    }
}
