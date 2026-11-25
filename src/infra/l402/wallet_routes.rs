use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use bitcoin::hashes::{sha256, Hash};
use reqwest::{Client, Url};
use std::{collections::BTreeMap, num::NonZeroU32, sync::Arc, time::Duration};
use swissknife_l402::{require_payment, Bakery, GateError, PaidRoute};
use swissknife_types::MerchantOrigin;
use uuid::Uuid;

use super::{
    config::MerchantConfig,
    get_paid_resource,
    merchant::{ChallengeLimit, Merchant},
    pricing::Price,
    Target,
};
use crate::{
    application::{composition::AppServices, errors::WebServerError},
    domains::l402::{merchant_handler, valid_name, MerchantCatalog},
    infra::axum::Path,
};

struct Origin {
    target: Target,
    quote: Option<Url>,
    maximum_msat: u64,
    seconds: u32,
    allowance: Option<NonZeroU32>,
    scope: String,
    challenges: Arc<ChallengeLimit>,
}

struct WalletGateway {
    catalog: Arc<MerchantCatalog>,
    origins: BTreeMap<(Uuid, String), Origin>,
    bakery: Bakery,
    services: Arc<AppServices>,
}

pub(super) fn router(
    config: &MerchantConfig,
    client: &Client,
    services: Arc<AppServices>,
    bakery: Bakery,
) -> Result<Router<Arc<AppServices>>, WebServerError> {
    let mut approved = BTreeMap::new();
    let mut origins = BTreeMap::new();
    for (name, spec) in &config.origins {
        let (url, maximum_msat, quote) = spec.validate(name).map_err(WebServerError::Merchant)?;
        let key = (spec.wallet_id, name.clone());
        approved.insert(
            key.clone(),
            MerchantOrigin {
                name: name.clone(),
                maximum_price_msat: maximum_msat,
                uses_quote: quote.is_some(),
                access_seconds: spec.access_seconds,
                requests_per_payment: spec.requests_per_payment.map(NonZeroU32::get),
            },
        );
        let binding = serde_json::to_vec(&(
            "wallet-merchant-v1",
            spec.wallet_id,
            name,
            url.as_str(),
            quote.as_ref().map(Url::as_str),
            spec.access_seconds,
            spec.requests_per_payment,
        ))
        .map_err(|_| WebServerError::Merchant("could not bind merchant origin"))?;
        origins.insert(
            key,
            Origin {
                target: Target {
                    client: client.clone(),
                    url,
                },
                quote,
                maximum_msat,
                seconds: spec.access_seconds,
                allowance: spec.requests_per_payment,
                scope: sha256::Hash::hash(&binding).to_string(),
                challenges: Arc::new(ChallengeLimit::new(spec.challenges_per_minute)),
            },
        );
    }
    let catalog = Arc::new(MerchantCatalog {
        origins: approved,
        routes: services.merchant_routes.clone(),
    });
    let state = Arc::new(WalletGateway {
        catalog: catalog.clone(),
        origins,
        bakery,
        services,
    });
    let protected = get(get_paid_resource)
        .route_layer(middleware::from_fn_with_state(state, paid_wallet_route))
        .head(|| async { StatusCode::METHOD_NOT_ALLOWED });
    Ok(merchant_handler::router(catalog).route("/l402/w/{wallet_id}/{name}", protected))
}

/// Download a paid resource published by the selected wallet.
#[utoipa::path(get,path="/l402/w/{wallet_id}/{name}",tag="L402",security((),("l402"=[])),
params(("wallet_id"=Uuid,Path),("name"=String,Path)),responses((status=200,body=Vec<u8>,content_type="application/octet-stream"),
(status=401,description="Invalid payment proof"),(status=402,description="A new Lightning invoice is required",headers(("WWW-Authenticate"=String,description="L402 challenge"))),
(status=404,description="Unknown resource or new sales are paused"),(status=429),(status=502),(status=503)))]
async fn paid_wallet_route(
    State(gateway): State<Arc<WalletGateway>>,
    Path((wallet, name)): Path<(Uuid, String)>,
    mut request: Request,
    next: Next,
) -> Result<Response, GateError> {
    if !valid_name(&name, 57) {
        return Err(GateError::Closed);
    }
    let terms = gateway
        .catalog
        .routes
        .get(wallet, &name)
        .await
        .map_err(|_| GateError::Unavailable)?
        .ok_or(GateError::Closed)?;
    let origin = gateway.origins.get(&(wallet, terms.origin)).ok_or(GateError::Closed)?;
    request.extensions_mut().insert(origin.target.clone());
    let route = PaidRoute {
        scope: format!("{}/{}", origin.scope, name),
        validity: Duration::from_secs(u64::from(origin.seconds)),
        bakery: gateway.bakery.clone(),
        gate: Arc::new(Merchant {
            services: gateway.services.clone(),
            name: format!("wallet/{name}"),
            wallet_id: wallet,
            enabled: terms.enabled && terms.price_msat > 0 && terms.price_msat <= origin.maximum_msat,
            price: Price::new(terms.price_msat, origin.quote.clone(), &origin.target.client),
            requests_per_payment: origin.allowance,
            invoice_seconds: (origin.seconds / 2).min(120),
            challenges: origin.challenges.clone(),
        }),
    };
    require_payment(State(Arc::new(route)), request, next).await
}
