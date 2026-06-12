use std::future::Future;
use std::time::Duration;

use tokio::time::{sleep, Instant};

/// Poll `check` until it returns true or `timeout` elapses, panicking with
/// `describe` on timeout.
///
/// SwissKnife exposes no client-facing event stream (no websocket/SSE), so a
/// client — and therefore these tests — observes asynchronous settlement
/// (incoming payments, on-chain confirmations) by polling the API. Centralizing
/// the loop keeps every such wait consistent. A server-push mechanism would let
/// clients react without polling; that is a SwissKnife enhancement tracked
/// separately.
pub async fn wait_until<F, Fut>(timeout: Duration, describe: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let started = Instant::now();
    loop {
        if check().await {
            return;
        }
        assert!(
            started.elapsed() < timeout,
            "timed out after {timeout:?} waiting for: {describe}"
        );
        sleep(Duration::from_millis(500)).await;
    }
}
