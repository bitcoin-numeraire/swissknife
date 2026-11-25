//! Protect an Axum route with a signed L402 macaroon and a paid invoice preimage.
mod bakery;
pub use bakery::{Bakery, InvalidCredential, PaidGrant};

use async_trait::async_trait;
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
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
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Busy => StatusCode::TOO_MANY_REQUESTS,
        };
        (status, [(header::CACHE_CONTROL, "private, no-store")]).into_response()
    }
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
    if let Some(grant) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|value| route.bakery.verify(&route.scope, value, now).ok())
    {
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

#[cfg(test)]
mod tests;
