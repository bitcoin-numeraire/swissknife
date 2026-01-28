use std::sync::Arc;

use axum::extract::State;
use utoipa::OpenApi;

use crate::{
    application::{
        docs::{INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE},
        dtos::{ErrorResponse, NostrNIP05QueryParams, NostrNIP05Response},
        entities::AppServices,
        errors::ApplicationError,
    },
    infra::axum::{Json, Query},
};

#[derive(OpenApi)]
#[openapi(
    paths(well_known_nostr),
    components(schemas(NostrNIP05Response)),
    tags(
        (name = "Nostr", description = "Public Nostr endpoints as defined in the [protocol specification](https://github.com/nostr-protocol/nips). Allows any Nostr client to identify a user's public keys")
    ),
)]
pub struct NostrHandler;

/// Well-known endpoint
///
/// Returns the names known by this service given username. The returned payload contains public keys in hex format. See [NIP-05](https://github.com/nostr-protocol/nips/blob/master/05.md)
#[utoipa::path(
    get,
    path = "/nostr.json",
    tag = "Nostr",
    context_path = "/.well-known",
    params(NostrNIP05QueryParams),
    responses(
        (status = 200, description = "Found", body = NostrNIP05Response),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
pub async fn well_known_nostr(
    Query(query_params): Query<NostrNIP05QueryParams>,
    State(services): State<Arc<AppServices>>,
) -> Result<Json<NostrNIP05Response>, ApplicationError> {
    let pubkey = services.nostr.get_pubkey(query_params.name.clone()).await?;
    Ok(NostrNIP05Response::new(query_params.name, pubkey).into())
}
