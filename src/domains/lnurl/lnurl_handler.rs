use std::sync::Arc;

use axum::{extract::State, routing::get, Router};
use utoipa::OpenApi;

use crate::{
    application::{
        docs::{BAD_REQUEST_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNPROCESSABLE_EXAMPLE},
        dtos::{ErrorResponse, LNUrlpInvoiceQueryParams, LnUrlCallbackResponse},
        entities::AppServices,
        errors::ApplicationError,
    },
    infra::axum::{Json, Path, Query},
};

use super::LnURLPayRequest;

#[derive(OpenApi)]
#[openapi(
    paths(well_known, callback),
    components(schemas(LnURLPayRequest, LnUrlCallbackResponse)),
    tags(
        (name = "LNURL", description = "Public LNURL endpoints as defined in the [protocol specification](https://github.com/lnurl/luds). Allows any active Lightning Address to receive payments")
    ),
)]
pub struct LnURLHandler;

pub fn router() -> Router<Arc<AppServices>> {
    Router::new().route("/:username/callback", get(callback))
}

/// Well-known endpoint
///
/// Returns the LNURL payRequest for this LN Address (username). The returned payload contains information allowing the payer to generate an invoice. See [LUDS-06](https://github.com/lnurl/luds/blob/luds/06.md)
#[utoipa::path(
    get,
    path = "/{username}",
    tag = "LNURL",
    context_path = "/.well-known/lnurlp",
    responses(
        (status = 200, description = "Found", body = LnURLPayRequest),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
pub async fn well_known(
    Path(username): Path<String>,
    State(services): State<Arc<AppServices>>,
) -> Result<Json<LnURLPayRequest>, ApplicationError> {
    let lnurlp = services.lnurl.lnurlp(username).await?;
    Ok(lnurlp.into())
}

/// LNURL callback endpoint
///
/// Returns the callback response for this LN Address (username). Containing an invoice and information on how to behave upon success. See [LUDS-06](https://github.com/lnurl/luds/blob/luds/06.md)
#[utoipa::path(
    get,
    path = "/{username}/callback",
    tag = "LNURL",
    context_path = "/lnurlp",
    params(LNUrlpInvoiceQueryParams),
    responses(
        (status = 200, description = "Found", body = LnUrlCallbackResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn callback(
    Path(username): Path<String>,
    Query(query_params): Query<LNUrlpInvoiceQueryParams>,
    State(services): State<Arc<AppServices>>,
) -> Result<Json<LnUrlCallbackResponse>, ApplicationError> {
    let callback = services
        .lnurl
        .lnurlp_callback(username, query_params.amount, query_params.comment)
        .await?;
    Ok(Json(callback.into()))
}
