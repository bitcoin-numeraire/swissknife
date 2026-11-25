//! Protect an Axum route with a signed L402 macaroon and a paid invoice preimage.
mod bakery;
pub use bakery::{Bakery, InvalidCredential, PaidGrant};

use async_trait::async_trait;
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct ChallengeInvoice {
    pub payment_hash: [u8; 32],
    pub bolt11: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GateError {
    #[error("Resource is unavailable for new sales")]
    Closed,
    #[error("Payment service unavailable")]
    Unavailable,
    #[error("Too many outstanding payment requests")]
    Busy,
}

/// The invoice and redemption policy belong to this merchant route. Single-use
/// redemption must claim the payment atomically across all serving replicas.
#[async_trait]
pub trait InvoiceGate: Send + Sync {
    async fn invoice(&self) -> Result<ChallengeInvoice, GateError>;
    async fn redeem(&self, grant: &PaidGrant) -> Result<bool, GateError>;
    /// Snapshot of uses remaining. None denotes an unmetered time pass.
    /// Gates that do not support passive inspection leave the result unavailable.
    async fn remaining_requests(&self, _grant: &PaidGrant) -> Result<Option<u64>, GateError> {
        Err(GateError::Unavailable)
    }
    /// For read-only resources, atomically claim usage after preparing a successful
    /// response. A competing request may already have claimed the last use.
    async fn record_success(&self, _grant: &PaidGrant) -> Result<bool, GateError> {
        Ok(true)
    }
}

pub struct PaidRoute {
    pub scope: String,
    pub validity: Duration,
    pub bakery: Bakery,
    pub gate: Arc<dyn InvoiceGate>,
}

impl IntoResponse for GateError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Closed => StatusCode::NOT_FOUND,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Busy => StatusCode::TOO_MANY_REQUESTS,
        };
        (status, [(header::CACHE_CONTROL, "private, no-store")]).into_response()
    }
}

/// Inspect an attenuated pass without a preimage. This never issues an invoice,
/// claims a request or calls the paid resource.
pub async fn pass_status(State(route): State<Arc<PaidRoute>>, headers: HeaderMap) -> Result<Response, GateError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| GateError::Unavailable)?
        .as_secs();
    let mut values = headers.get_all("swissknife-l402-inspection").iter();
    let token = values.next().and_then(|h| h.to_str().ok());
    let inspected = match (token, values.next()) {
        (Some(token), None) => route.bakery.inspect_status(&route.scope, token, now),
        _ => Err(InvalidCredential),
    };
    let (code, state, remaining) = match inspected {
        Ok(Some(grant)) => {
            let remaining = route.gate.remaining_requests(&grant).await?;
            let state = if remaining == Some(0) { "exhausted" } else { "active" };
            (StatusCode::OK, state, remaining)
        }
        Ok(None) => (StatusCode::OK, "inactive", None),
        Err(_) => (StatusCode::UNAUTHORIZED, "invalid", None),
    };
    let mut response = (
        code,
        [
            (header::CACHE_CONTROL.as_str(), "private, no-store"),
            ("swissknife-l402-status", state),
        ],
    )
        .into_response();
    if let Some(count) = remaining {
        response.headers_mut().insert("swissknife-l402-remaining", count.into());
    }
    Ok(response)
}

/// Install with `axum::middleware::from_fn_with_state`. This middleware strips
/// the client's payment proof before invoking the application.
pub async fn require_payment(
    State(route): State<Arc<PaidRoute>>,
    mut request: Request,
    next: Next,
) -> Result<Response, GateError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| GateError::Unavailable)?
        .as_secs();
    let grant = match payment_header(request.headers()) {
        Ok(Some(header)) => route.bakery.inspect(&route.scope, &header, now),
        Ok(None) => Ok(None),
        Err(error) => Err(error),
    };
    let grant = match grant {
        Ok(grant) => grant,
        Err(_) => {
            return Ok((
                StatusCode::UNAUTHORIZED,
                [
                    (header::WWW_AUTHENTICATE, "L402"),
                    (header::CACHE_CONTROL, "private, no-store"),
                ],
            )
                .into_response())
        }
    };
    if let Some(grant) = grant {
        if route.gate.redeem(&grant).await? {
            request.headers_mut().remove(header::AUTHORIZATION);
            request.extensions_mut().insert(grant.clone());
            let mut response = next.run(request).await;
            if !response.status().is_success() || route.gate.record_success(&grant).await? {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
                return Ok(response);
            }
        }
    }
    let invoice = route.gate.invoice().await?;
    let macaroon = route
        .bakery
        .mint(
            &route.scope,
            invoice.payment_hash,
            now.checked_add(route.validity.as_secs())
                .ok_or(GateError::Unavailable)?,
        )
        .map_err(|_| GateError::Unavailable)?;
    if !invoice.bolt11.bytes().all(|c| c.is_ascii_alphanumeric()) || invoice.bolt11.is_empty() {
        return Err(GateError::Unavailable);
    }
    let challenge = HeaderValue::from_str(&format!("L402 macaroon=\"{macaroon}\", invoice=\"{}\"", invoice.bolt11))
        .map_err(|_| GateError::Unavailable)?;
    let mut response = StatusCode::PAYMENT_REQUIRED.into_response();
    response.headers_mut().insert(header::WWW_AUTHENTICATE, challenge);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    Ok(response)
}

// HTTP authentication schemes are case-insensitive. Normalize the supported
// schemes for the protocol parser and leave other authentication schemes alone.
fn payment_header(headers: &axum::http::HeaderMap) -> Result<Option<String>, InvalidCredential> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let Some(header) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(InvalidCredential);
    }
    let bytes = header.as_bytes().trim_ascii();
    let scheme_end = bytes.iter().position(u8::is_ascii_whitespace).unwrap_or(bytes.len());
    let scheme = &bytes[..scheme_end];
    if !scheme.eq_ignore_ascii_case(b"L402") && !scheme.eq_ignore_ascii_case(b"LSAT") {
        return Ok(None);
    }
    if bytes.len() > 8192 {
        return Err(InvalidCredential);
    }
    let value = std::str::from_utf8(bytes[scheme_end..].trim_ascii()).map_err(|_| InvalidCredential)?;
    Ok(Some(format!("L402 {value}")))
}

#[cfg(test)]
mod tests;
