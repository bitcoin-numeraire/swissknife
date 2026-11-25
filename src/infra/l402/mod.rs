mod config;
mod merchant;
mod pricing;
mod wallet_routes;

pub use config::MerchantConfig;
use merchant::{ChallengeLimit, Merchant};
use pricing::Price;

use std::{sync::Arc, time::Duration};

use axum::{
    extract::Extension,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, head},
    Router,
};
use bitcoin::hashes::{sha256, Hash};
use bytes::BytesMut;
use reqwest::{redirect::Policy, Client, Url};
use swissknife_l402::{pass_status, require_payment, Bakery, PaidRoute};
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
    let Some(config) = config else {
        return Ok(crate::domains::l402::merchant_handler::router(Arc::new(
            crate::domains::l402::MerchantCatalog {
                origins: Default::default(),
                routes: services.merchant_routes.clone(),
            },
        )));
    };
    let bakery = Bakery::new(&config.root_key);
    let client = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| WebServerError::Merchant("could not create upstream client"))?;

    router = router.merge(wallet_routes::router(
        config,
        &client,
        services.clone(),
        bakery.clone(),
    )?);
    for (name, route) in &config.routes {
        let (url, price_msat, quote_url) = route.validate(name).map_err(WebServerError::Merchant)?;
        let terms = (name, route.wallet_id, url.as_str(), price_msat, route.access_seconds);
        // Retain the original scope for already-paid time passes.
        let binding = match (&quote_url, route.requests_per_payment) {
            (Some(quote), limit) => serde_json::to_vec(&("merchant-quote-v1", terms, quote.as_str(), limit)),
            (None, Some(limit)) => serde_json::to_vec(&(terms, limit)),
            (None, None) => serde_json::to_vec(&terms),
        }
        .map_err(|_| WebServerError::Merchant("could not bind L402 route scope"))?;
        let paid = Arc::new(PaidRoute {
            scope: sha256::Hash::hash(&binding).to_string(),
            validity: Duration::from_secs(u64::from(route.access_seconds)),
            bakery: bakery.clone(),
            gate: Arc::new(Merchant {
                services: services.clone(),
                name: name.clone(),
                wallet_id: route.wallet_id,
                enabled: true,
                price: Price::new(price_msat, quote_url, &client),
                requests_per_payment: route.requests_per_payment,
                invoice_seconds: (route.access_seconds / 2).min(120),
                challenges: Arc::new(ChallengeLimit::new(route.challenges_per_minute)),
            }),
        });
        let status = head(merchant_pass_status).with_state(paid.clone());
        router = router.route(&format!("/l402/{name}/status"), status);
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
#[openapi(paths(get_paid_resource,merchant_pass_status,wallet_routes::paid_wallet_route,wallet_routes::wallet_pass_status), tags((name = "L402", description = "Wallet-backed paid resources")))]
pub struct MerchantHandler;

/// Inspect this merchant's cached pass using a status-only macaroon.
#[utoipa::path(head, path="/l402/{name}/status", tag="L402",
    params(("name"=String, Path),("SwissKnife-L402-Inspection"=String,Header,description="Macaroon attenuated with purpose=status")),
    responses((status=200,description="Pass terms at the time of inspection",
        headers(("SwissKnife-L402-Status"=String,description="active, exhausted or inactive"),("SwissKnife-L402-Remaining"=u64,description="Remaining successful requests for a metered pass"))),
    (status=401,description="Invalid status credential"),(status=404),(status=503)))]
async fn merchant_pass_status(
    state: axum::extract::State<Arc<PaidRoute>>,
    headers: HeaderMap,
) -> Result<Response, swissknife_l402::GateError> {
    pass_status(state, headers).await
}

/// Fetch a fixed merchant resource with an L402 access pass. Each configured
/// route advertises its invoice in WWW-Authenticate. A paid pass can be reused
/// until expiry or exhaustion of its configured request allowance.
#[utoipa::path(get, path="/l402/{name}", tag="L402", security((), ("l402" = [])),
    params(("name" = String, Path, description = "Operator-configured merchant route")),
    responses((status=200, description="Paid resource", content_type="application/octet-stream", body=Vec<u8>),
        (status=401, description="Invalid L402 credential or payment proof"),
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
