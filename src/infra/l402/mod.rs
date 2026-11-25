mod config;
mod merchant;

pub use config::MerchantConfig;
use merchant::{ChallengeLimit, Merchant};

use std::{sync::Arc, time::Duration};

use axum::{
    extract::Extension,
    http::{header, HeaderValue, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use bitcoin::hashes::{sha256, Hash};
use bytes::BytesMut;
use reqwest::{redirect::Policy, Client, Url};
use swissknife_l402::{require_payment, Bakery, PaidRoute};
use utoipa::OpenApi;

use crate::application::{composition::AppServices, errors::WebServerError};

const MAX_BODY: usize = 2 * 1024 * 1024;

#[derive(Clone)]
struct Target {
    client: Client,
    url: Url,
}

pub fn router(
    config: Option<&MerchantConfig>,
    services: Arc<AppServices>,
) -> Result<Router<Arc<AppServices>>, WebServerError> {
    let mut router = Router::new();
    let Some(config) = config else { return Ok(router) };
    let bakery = Bakery::new(&config.root_key);
    let client = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| WebServerError::Merchant("could not create upstream client"))?;

    for (name, route) in &config.routes {
        let (url, price_msat) = route.validate(name).map_err(WebServerError::Merchant)?;
        let binding = serde_json::to_vec(&(name, route.wallet_id, url.as_str(), price_msat, route.access_seconds))
            .map_err(|_| WebServerError::Merchant("could not bind L402 route scope"))?;
        let paid = Arc::new(PaidRoute {
            scope: sha256::Hash::hash(&binding).to_string(),
            validity: Duration::from_secs(u64::from(route.access_seconds)),
            bakery: bakery.clone(),
            gate: Arc::new(Merchant {
                services: services.clone(),
                name: name.clone(),
                wallet_id: route.wallet_id,
                price_msat,
                invoice_seconds: (route.access_seconds / 2).min(120),
                challenges: ChallengeLimit::new(route.challenges_per_minute),
            }),
        });
        let handler = get(get_paid_resource)
            .layer::<_, std::convert::Infallible>(Extension(Target {
                client: client.clone(),
                url,
            }))
            .route_layer(middleware::from_fn_with_state(paid, require_payment))
            .head(|| async { StatusCode::METHOD_NOT_ALLOWED });
        router = router.route(&format!("/l402/{name}"), handler);
    }
    Ok(router)
}

#[derive(OpenApi)]
#[openapi(paths(get_paid_resource), tags((name = "L402", description = "Wallet-backed paid resources")))]
pub struct MerchantHandler;

/// Fetch a fixed merchant resource with an L402 access pass. Each configured
/// route advertises its invoice in WWW-Authenticate. A paid pass can be reused
/// until the expiry set when its challenge was issued.
#[utoipa::path(get, path="/l402/{name}", tag="L402", security((), ("l402" = [])),
    params(("name" = String, Path, description = "Operator-configured merchant route")),
    responses((status=200, description="Paid resource", content_type="application/octet-stream", body=Vec<u8>),
        (status=402, description="Pay the advertised invoice and present the macaroon and preimage",
            headers(("WWW-Authenticate" = String, description = "L402 macaroon and BOLT11 invoice"))),
        (status=404, description="Unknown merchant route"), (status=429, description="Invoice issuance limit reached"),
        (status=502, description="Upstream failed, redirected or exceeded the response limit"),
        (status=503, description="Invoice service unavailable")))]
async fn get_paid_resource(Extension(target): Extension<Target>) -> Result<Response, StatusCode> {
    let mut upstream = target
        .client
        .get(target.url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !upstream.status().is_success() || upstream.content_length().is_some_and(|size| size > MAX_BODY as u64) {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or(HeaderValue::from_static("application/octet-stream"));
    let status = upstream.status();
    let mut body = BytesMut::new();
    while let Some(chunk) = upstream.chunk().await.map_err(|_| StatusCode::BAD_GATEWAY)? {
        if chunk.len() > MAX_BODY - body.len() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        body.extend_from_slice(&chunk);
    }
    Ok((
        status,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, HeaderValue::from_static("attachment")),
            (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
            (
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static("sandbox; default-src 'none'"),
            ),
        ],
        body.freeze(),
    )
        .into_response())
}
