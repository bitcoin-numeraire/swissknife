use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use futures_util::{stream, StreamExt};
use hmac::{Hmac, Mac};
use reqwest::{redirect::Policy, StatusCode, Url};
use serde::Serialize;
use sha2::Sha256;
use tokio::{sync::watch, task::JoinHandle};
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::{
    application::composition::AppStore,
    domains::event::{ClaimedWebhookDelivery, ClientEventType},
};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const LEASE_SECONDS: i64 = 60;
const PREPARE_BATCH_SIZE: u64 = 100;
const DELIVERY_BATCH_SIZE: u64 = 20;
const DELIVERY_CONCURRENCY: usize = 10;
const MAX_ATTEMPTS: u32 = 8;
const MAX_ERROR_LENGTH: usize = 2_000;

type HmacSha256 = Hmac<Sha256>;

pub struct WebhookWorker {
    store: AppStore,
}

impl WebhookWorker {
    pub fn new(store: AppStore) -> Self {
        Self { store }
    }

    pub fn start(self, shutdown: watch::Receiver<bool>) -> JoinHandle<()> {
        tokio::spawn(async move { self.run(shutdown).await })
    }

    async fn run(self, mut shutdown: watch::Receiver<bool>) {
        loop {
            if *shutdown.borrow() {
                break;
            }

            if let Err(error) = self.run_cycle().await {
                error!(%error, "Webhook delivery cycle failed");
            }

            tokio::select! {
                result = shutdown.changed() => {
                    if result.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(POLL_INTERVAL) => {}
            }
        }

        debug!("Webhook delivery worker stopped");
    }

    async fn run_cycle(&self) -> anyhow::Result<()> {
        let prepared = self.store.webhook.prepare_deliveries(PREPARE_BATCH_SIZE).await?;
        let now = Utc::now();
        let deliveries = self
            .store
            .webhook
            .claim_due(now, now + chrono::Duration::seconds(LEASE_SECONDS), DELIVERY_BATCH_SIZE)
            .await?;

        if prepared > 0 || !deliveries.is_empty() {
            debug!(
                prepared,
                claimed = deliveries.len(),
                "Processing durable webhook deliveries"
            );
        }

        stream::iter(deliveries)
            .for_each_concurrent(DELIVERY_CONCURRENCY, |delivery| async move {
                self.deliver(delivery).await;
            })
            .await;
        Ok(())
    }

    async fn deliver(&self, delivery: ClaimedWebhookDelivery) {
        let result = send_delivery(&delivery).await;
        match result {
            Ok(status) => {
                if let Err(error) = self.store.webhook.mark_delivered(delivery.id, status.as_u16()).await {
                    error!(%error, delivery_id = %delivery.id, "Failed to record successful webhook delivery");
                }
            }
            Err(failure) => {
                let attempt = delivery.attempt_count + 1;
                let exhausted = failure.permanent || attempt >= MAX_ATTEMPTS;
                let next_attempt_at = Utc::now() + retry_delay(delivery.attempt_count);
                let error_message = truncate_error(failure.message);
                if let Err(error) = self
                    .store
                    .webhook
                    .mark_failed(
                        delivery.id,
                        failure.status.map(|status| status.as_u16()),
                        error_message.clone(),
                        next_attempt_at,
                        exhausted,
                    )
                    .await
                {
                    error!(%error, delivery_id = %delivery.id, "Failed to record webhook delivery failure");
                } else if exhausted {
                    warn!(
                        delivery_id = %delivery.id,
                        subscription_id = %delivery.subscription_id,
                        attempts = attempt,
                        error = %error_message,
                        "Webhook delivery exhausted"
                    );
                }
            }
        }
    }
}

#[derive(Serialize)]
struct WebhookEnvelope<'a> {
    id: &'a str,
    #[serde(rename = "type")]
    event_type: ClientEventType,
    wallet_id: Uuid,
    resource_id: Uuid,
    created_at: chrono::DateTime<Utc>,
    data: &'a serde_json::Value,
}

#[derive(Debug)]
struct DeliveryFailure {
    status: Option<StatusCode>,
    message: String,
    permanent: bool,
}

async fn send_delivery(delivery: &ClaimedWebhookDelivery) -> Result<StatusCode, DeliveryFailure> {
    let url = validate_and_resolve_url(&delivery.url).await?;
    let timestamp = Utc::now().timestamp();
    let (body, signature) = signed_payload(delivery, timestamp)?;

    let host = url.host_str().expect("validated webhook URL has a host");
    let port = url.port_or_known_default().unwrap_or(443);
    let ip = resolve_public_ip(host, port).await?;
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(REQUEST_TIMEOUT)
        .resolve(host, SocketAddr::new(ip, port))
        .build()
        .map_err(|error| DeliveryFailure {
            status: None,
            message: format!("Failed to build webhook HTTP client: {error}"),
            permanent: false,
        })?;

    post_webhook(&client, url, delivery, body, timestamp, signature).await
}

fn signed_payload(delivery: &ClaimedWebhookDelivery, timestamp: i64) -> Result<(Vec<u8>, String), DeliveryFailure> {
    let body = serde_json::to_vec(&WebhookEnvelope {
        id: &delivery.event.id,
        event_type: delivery.event.event_type,
        wallet_id: delivery.event.wallet_id,
        resource_id: delivery.event.resource_id,
        created_at: delivery.event.created_at,
        data: &delivery.event.data,
    })
    .map_err(|error| DeliveryFailure {
        status: None,
        message: format!("Failed to serialize webhook body: {error}"),
        permanent: true,
    })?;
    let signature = sign_payload(&delivery.signing_secret, timestamp, &body).map_err(|message| DeliveryFailure {
        status: None,
        message,
        permanent: true,
    })?;
    Ok((body, signature))
}

async fn post_webhook(
    client: &reqwest::Client,
    url: Url,
    delivery: &ClaimedWebhookDelivery,
    body: Vec<u8>,
    timestamp: i64,
    signature: String,
) -> Result<StatusCode, DeliveryFailure> {
    let response = client
        .post(url)
        .header("content-type", "application/json")
        .header("x-swissknife-event", delivery.event.event_type.to_string())
        .header("x-swissknife-delivery", delivery.id.to_string())
        .header("x-swissknife-timestamp", timestamp.to_string())
        .header("x-swissknife-signature", format!("v1={signature}"))
        .body(body)
        .send()
        .await
        .map_err(|error| DeliveryFailure {
            status: None,
            message: format!("Webhook request failed: {error}"),
            permanent: false,
        })?;
    let status = response.status();
    if status.is_success() {
        return Ok(status);
    }

    Err(DeliveryFailure {
        status: Some(status),
        message: format!("Webhook endpoint returned HTTP {status}"),
        permanent: !is_retryable_status(status),
    })
}

async fn validate_and_resolve_url(raw: &str) -> Result<Url, DeliveryFailure> {
    let url = Url::parse(raw).map_err(|error| DeliveryFailure {
        status: None,
        message: format!("Invalid webhook URL: {error}"),
        permanent: true,
    })?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(DeliveryFailure {
            status: None,
            message: "Webhook URL must be public HTTPS without credentials or a fragment".to_string(),
            permanent: true,
        });
    }

    let host = url.host_str().expect("checked above");
    resolve_public_ip(host, url.port_or_known_default().unwrap_or(443)).await?;
    Ok(url)
}

async fn resolve_public_ip(host: &str, port: u16) -> Result<IpAddr, DeliveryFailure> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return if is_public_ip(ip) {
            Ok(ip)
        } else {
            Err(private_address_failure())
        };
    }

    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| DeliveryFailure {
            status: None,
            message: format!("Webhook DNS lookup failed: {error}"),
            permanent: false,
        })?;
    let mut public_ip = None;
    for address in addresses {
        if !is_public_ip(address.ip()) {
            return Err(private_address_failure());
        }
        public_ip.get_or_insert(address.ip());
    }

    public_ip.ok_or_else(|| DeliveryFailure {
        status: None,
        message: "Webhook hostname resolved to no addresses".to_string(),
        permanent: false,
    })
}

fn private_address_failure() -> DeliveryFailure {
    DeliveryFailure {
        status: None,
        message: "Webhook hostname resolves to a private, local, or reserved address".to_string(),
        permanent: true,
    }
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_broadcast()
        || a == 0
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 240)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        return is_public_ipv4(ipv4);
    }
    let segments = ip.segments();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8))
}

fn sign_payload(secret: &str, timestamp: i64, body: &[u8]) -> Result<String, String> {
    let key = URL_SAFE_NO_PAD
        .decode(secret)
        .map_err(|error| format!("Invalid webhook signing secret: {error}"))?;
    let mut signer = HmacSha256::new_from_slice(&key).map_err(|error| error.to_string())?;
    signer.update(timestamp.to_string().as_bytes());
    signer.update(b".");
    signer.update(body);
    Ok(hex::encode(signer.finalize().into_bytes()))
}

fn is_retryable_status(status: StatusCode) -> bool {
    status.is_server_error()
        || matches!(
            status,
            StatusCode::REQUEST_TIMEOUT | StatusCode::CONFLICT | StatusCode::TOO_EARLY | StatusCode::TOO_MANY_REQUESTS
        )
}

fn retry_delay(attempt_count: u32) -> chrono::Duration {
    let seconds = 60_i64.saturating_mul(1_i64 << attempt_count.min(6)).min(3_600);
    chrono::Duration::seconds(seconds)
}

fn truncate_error(mut error: String) -> String {
    if error.len() > MAX_ERROR_LENGTH {
        error.truncate(MAX_ERROR_LENGTH);
    }
    error
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn delivery() -> ClaimedWebhookDelivery {
        ClaimedWebhookDelivery {
            id: Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap(),
            subscription_id: Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").unwrap(),
            event: crate::domains::event::ClientEvent {
                id: "42".to_string(),
                event_type: ClientEventType::PaymentSettled,
                wallet_id: Uuid::parse_str("cccccccc-cccc-4ccc-8ccc-cccccccccccc").unwrap(),
                resource_id: Uuid::parse_str("dddddddd-dddd-4ddd-8ddd-dddddddddddd").unwrap(),
                data: json!({"status": "Settled"}),
                created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            },
            url: "https://hooks.example.com/swissknife".to_string(),
            signing_secret: URL_SAFE_NO_PAD.encode(b"test secret"),
            attempt_count: 0,
        }
    }

    #[test]
    fn signs_timestamp_dot_raw_body() {
        let secret = URL_SAFE_NO_PAD.encode(b"test secret");
        assert_eq!(
            sign_payload(&secret, 1_700_000_000, br#"{"id":"42"}"#).unwrap(),
            "7be8775da27f23f69337327814786d31f61db99e81fd85e20a6be3f1dd3f3c71"
        );
    }

    #[test]
    fn rejects_local_and_reserved_addresses() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.1.1",
            "192.0.2.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
        ] {
            assert!(!is_public_ip(ip.parse().unwrap()), "{ip} must not be accepted");
        }
        assert!(is_public_ip("8.8.8.8".parse().unwrap()));
        assert!(is_public_ip("2606:4700:4700::1111".parse().unwrap()));
    }

    #[test]
    fn retries_only_transient_http_statuses() {
        for status in [408, 409, 425, 429, 500, 503] {
            assert!(is_retryable_status(StatusCode::from_u16(status).unwrap()));
        }
        for status in [301, 400, 401, 404, 422] {
            assert!(!is_retryable_status(StatusCode::from_u16(status).unwrap()));
        }
    }

    #[test]
    fn retry_backoff_caps_at_one_hour() {
        assert_eq!(retry_delay(0), chrono::Duration::minutes(1));
        assert_eq!(retry_delay(3), chrono::Duration::minutes(8));
        assert_eq!(retry_delay(7), chrono::Duration::hours(1));
    }

    #[tokio::test]
    async fn posts_the_documented_signed_wire_contract() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        let delivery = delivery();
        let timestamp = 1_700_000_001;
        let (body, signature) = signed_payload(&delivery, timestamp).unwrap();

        let status = post_webhook(
            &reqwest::Client::new(),
            Url::parse(&format!("{}/hook", server.uri())).unwrap(),
            &delivery,
            body.clone(),
            timestamp,
            signature.clone(),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::NO_CONTENT);

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.body, body);
        assert_eq!(request.headers.get("x-swissknife-event").unwrap(), "payment.settled");
        assert_eq!(
            request.headers.get("x-swissknife-delivery").unwrap(),
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
        );
        assert_eq!(request.headers.get("x-swissknife-timestamp").unwrap(), "1700000001");
        assert_eq!(
            request.headers.get("x-swissknife-signature").unwrap(),
            format!("v1={signature}").as_str()
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
            json!({
                "id": "42",
                "type": "payment.settled",
                "wallet_id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
                "resource_id": "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
                "created_at": "2023-11-14T22:13:20Z",
                "data": {"status": "Settled"}
            })
        );
    }
}
