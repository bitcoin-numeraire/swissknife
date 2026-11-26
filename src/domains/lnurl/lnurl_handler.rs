use std::sync::Arc;

use axum::{
    extract::{rejection::PathRejection, Path as AxumPath, State},
    http::{header::CACHE_CONTROL, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use utoipa::OpenApi;
use uuid::Uuid;

use swissknife_types::{ErrorResponse, LNUrlpInvoiceQueryParams, LnUrlVerifyResponse};

use crate::{
    application::{
        composition::AppServices,
        docs::{BAD_REQUEST_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNPROCESSABLE_EXAMPLE},
        errors::ApplicationError,
    },
    infra::axum::{Json, Path, Query},
};

use super::{LnURLPayRequest, LnUrlCallback};

#[derive(OpenApi)]
#[openapi(
    paths(well_known, callback, verify),
    components(schemas(LnURLPayRequest, LnUrlCallback, LnUrlVerifyResponse)),
    tags(
        (name = "LNURL", description = "Public LNURL endpoints as defined in the [protocol specification](https://github.com/lnurl/luds). Allows any active Lightning Address to receive payments")
    ),
)]
pub struct LnURLHandler;

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/{username}/callback", get(callback))
        .route("/{token}/verify", get(verify))
}

/// Well-known endpoint
///
/// Returns the LNURL payRequest for this LN Address (username). The returned payload contains information allowing the payer to generate an invoice. See [LUDS-06](https://github.com/lnurl/luds/blob/luds/06.md)
#[utoipa::path(
    get,
    path = "/{username}",
    tag = "LNURL",
    context_path = "/.well-known/lnurlp",
    security(),
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
/// Returns an invoice, a success action, and a URL to verify settlement.
/// See [LUD-06](https://github.com/lnurl/luds/blob/luds/06.md) and [LUD-21](https://github.com/lnurl/luds/blob/luds/21.md).
#[utoipa::path(
    get,
    path = "/{username}/callback",
    tag = "LNURL",
    context_path = "/lnurlp",
    params(LNUrlpInvoiceQueryParams),
    security(),
    responses(
        (status = 200, description = "Found", body = LnUrlCallback),
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
) -> Result<impl IntoResponse, ApplicationError> {
    let callback = services
        .lnurl
        .lnurlp_callback(username, query_params.amount, query_params.comment)
        .await?;
    Ok(([(CACHE_CONTROL, "no-store")], Json(callback)))
}

/// Verify an LNURL-pay invoice
///
/// Check settlement using the URL from the callback response. The preimage
/// is returned only after settlement. See [LUD-21](https://github.com/lnurl/luds/blob/luds/21.md).
#[utoipa::path(
    get,
    path = "/{token}/verify",
    operation_id = "verify_lnurl_invoice",
    context_path = "/lnurlp",
    tag = "LNURL",
    security(),
    params(("token" = Uuid, Path, description = "Invoice verification token from the LNURL-pay callback")),
    responses(
        (status = 200, description = "Invoice verification result, or Not found for an invalid or unknown token", body = LnUrlVerifyResponse),
        (status = 500, description = "Verification unavailable", body = LnUrlVerifyResponse)
    )
)]
async fn verify(
    token: Result<AxumPath<Uuid>, PathRejection>,
    State(services): State<Arc<AppServices>>,
) -> impl IntoResponse {
    let verification = match token {
        Ok(AxumPath(token)) => services.lnurl.verify(token).await,
        Err(_) => Ok(LnUrlVerifyResponse::Error {
            reason: "Not found".into(),
        }),
    };
    let (status, response) = match verification {
        Ok(response) => (StatusCode::OK, response),
        Err(error) => {
            tracing::error!(%error, "Invoice verification failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                LnUrlVerifyResponse::Error {
                    reason: "Verification unavailable".into(),
                },
            )
        }
    };
    (status, [(CACHE_CONTROL, "no-store")], Json(response))
}

#[cfg(test)]
mod tests {
    use crate::application::{composition::MockAppServicesBuilder, errors::DataError};

    use super::*;

    mod well_known {
        use super::*;

        #[tokio::test]
        async fn forwards_the_username_and_propagates_not_found() {
            let mut builder = MockAppServicesBuilder::new();
            builder
                .lnurl
                .expect_lnurlp()
                .withf(|username| username == "alice")
                .times(1)
                .returning(|_| Err(DataError::NotFound("missing".to_string()).into()));

            let result = well_known(Path("alice".to_string()), State(Arc::new(builder.build()))).await;

            assert!(matches!(result, Err(ApplicationError::Data(DataError::NotFound(_)))));
        }
    }

    mod callback {
        use super::*;

        #[tokio::test]
        async fn forwards_amount_and_comment_to_the_service() {
            let mut builder = MockAppServicesBuilder::new();
            builder
                .lnurl
                .expect_lnurlp_callback()
                .withf(|username, amount, comment| {
                    username == "alice" && *amount == 2_000 && comment.as_deref() == Some("thanks")
                })
                .times(1)
                .returning(|_, _, _| Err(DataError::NotFound("missing".to_string()).into()));

            let result = callback(
                Path("alice".to_string()),
                Query(LNUrlpInvoiceQueryParams {
                    amount: 2_000,
                    comment: Some("thanks".to_string()),
                }),
                State(Arc::new(builder.build())),
            )
            .await;

            assert!(matches!(result, Err(ApplicationError::Data(DataError::NotFound(_)))));
        }
    }
}
