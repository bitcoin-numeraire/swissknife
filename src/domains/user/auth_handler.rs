use std::sync::Arc;

use axum::{extract::State, routing::post, Router};
use utoipa::OpenApi;

use crate::{
    application::{
        docs::{
            BAD_REQUEST_EXAMPLE, CONFLICT_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNSUPPORTED_EXAMPLE,
        },
        dtos::{ErrorResponse, SignInRequest, SignInResponse, SignUpRequest},
        errors::ApplicationError,
    },
    infra::{app::AppState, axum::Json},
};

#[derive(OpenApi)]
#[openapi(
    paths(sign_in, sign_up),
    components(schemas(SignUpRequest, SignInRequest, SignInResponse)),
    tags(
        (name = "Authentication", description = "Some endpoints are public, but some require authentication. We provide all the required endpoints to create an account and authorize yourself.")
    )
)]
pub struct AuthHandler;
pub const CONTEXT_PATH: &str = "/v1/auth";

pub fn auth_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sign-up", post(sign_up))
        .route("/sign-in", post(sign_in))
}

/// Sign up
///
/// Creates the initial Admin user. Returns a JWT token to be used for authentication. The JWT token contains authentication and permissions. Sign up is only available for `JWT` Auth provider.
#[utoipa::path(
    post,
    path = "/sign-up",
    tag = "Authentication",
    context_path = CONTEXT_PATH,
    request_body = SignUpRequest,
    responses(
        (status = 200, description = "Admin user created", body = SignInResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 409, description = "Duplicate", body = ErrorResponse, example = json!(CONFLICT_EXAMPLE)),
        (status = 405, description = "Unsupported", body = ErrorResponse, example = json!(UNSUPPORTED_EXAMPLE))
    )
)]
async fn sign_up(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SignUpRequest>,
) -> Result<Json<SignInResponse>, ApplicationError> {
    let token = app_state.services.auth.sign_up(payload.password).await?;
    Ok(SignInResponse { token }.into())
}

/// Sign In
///
/// Returns a JWT token to be used for authentication. The JWT token contains authentication and permissions. Sign in is only available for `JWT` Auth provider.
#[utoipa::path(
    post,
    path = "/sign-in",
    tag = "Authentication",
    context_path = CONTEXT_PATH,
    request_body = SignInRequest,
    responses(
        (status = 200, description = "Token Created", body = SignInResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 405, description = "Unsupported", body = ErrorResponse, example = json!(UNSUPPORTED_EXAMPLE))
    )
)]
async fn sign_in(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<SignInRequest>,
) -> Result<Json<SignInResponse>, ApplicationError> {
    let token = app_state.services.auth.sign_in(payload.password).await?;
    Ok(SignInResponse { token }.into())
}
