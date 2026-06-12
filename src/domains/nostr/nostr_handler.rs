use std::sync::Arc;

use axum::extract::State;
use utoipa::OpenApi;

use swissknife_types::{ErrorResponse, NostrNIP05QueryParams, NostrNIP05Response};

use crate::{
    application::{
        docs::{INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE},
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

#[cfg(test)]
mod tests {
    use nostr_sdk::PublicKey;

    use crate::application::{entities::MockAppServicesBuilder, errors::DataError};

    use super::*;

    // Generator point x-coordinate: a valid x-only (Schnorr) public key.
    const VALID_PUBKEY_HEX: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    fn query(name: &str) -> NostrNIP05QueryParams {
        NostrNIP05QueryParams { name: name.to_string() }
    }

    mod well_known_nostr {
        use super::*;

        #[tokio::test]
        async fn forwards_the_name_and_returns_the_pubkey() {
            let pubkey = PublicKey::from_hex(VALID_PUBKEY_HEX).unwrap();

            let mut builder = MockAppServicesBuilder::new();
            builder
                .nostr
                .expect_get_pubkey()
                .withf(|name| name == "alice")
                .times(1)
                .returning(move |_| Ok(pubkey));

            let result = well_known_nostr(Query(query("alice")), State(Arc::new(builder.build()))).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn propagates_not_found() {
            let mut builder = MockAppServicesBuilder::new();
            builder
                .nostr
                .expect_get_pubkey()
                .times(1)
                .returning(|_| Err(DataError::NotFound("missing".to_string()).into()));

            let result = well_known_nostr(Query(query("alice")), State(Arc::new(builder.build()))).await;

            assert!(matches!(result, Err(ApplicationError::Data(DataError::NotFound(_)))));
        }
    }
}
