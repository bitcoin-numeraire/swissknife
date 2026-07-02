use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Router};
use utoipa::OpenApi;

use swissknife_types::{ChangePasswordRequest, ErrorResponse, SignInRequest, SignInResponse, SignUpRequest};

use crate::{
    application::{
        composition::AppServices,
        docs::{BAD_REQUEST_EXAMPLE, CONFLICT_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE, UNSUPPORTED_EXAMPLE},
        errors::ApplicationError,
    },
    infra::axum::Json,
};

use super::User;

#[derive(OpenApi)]
#[openapi(
    paths(sign_in, sign_up, change_password),
    components(schemas(ChangePasswordRequest, SignUpRequest, SignInRequest, SignInResponse)),
    tags(
        (name = "Authentication", description = "Some endpoints are public, but some require authentication. We provide all the required endpoints to create an account and authorize yourself.")
    )
)]
pub struct AuthHandler;
pub const CONTEXT_PATH: &str = "/v1/auth";

pub fn auth_router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/sign-up", post(sign_up))
        .route("/sign-in", post(sign_in))
        .route("/change-password", post(change_password))
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
    State(services): State<Arc<AppServices>>,
    Json(payload): Json<SignUpRequest>,
) -> Result<Json<SignInResponse>, ApplicationError> {
    let token = services.auth.sign_up(payload.password).await?;
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
    State(services): State<Arc<AppServices>>,
    Json(payload): Json<SignInRequest>,
) -> Result<Json<SignInResponse>, ApplicationError> {
    let token = services.auth.sign_in(payload.password).await?;
    Ok(SignInResponse { token }.into())
}

/// Change Password
///
/// Changes the local owner password for `JWT` auth provider deployments.
#[utoipa::path(
    post,
    path = "/change-password",
    tag = "Authentication",
    context_path = CONTEXT_PATH,
    request_body = ChangePasswordRequest,
    responses(
        (status = 204, description = "Password changed"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 405, description = "Unsupported", body = ErrorResponse, example = json!(UNSUPPORTED_EXAMPLE))
    ),
    security(("jwt" = []))
)]
async fn change_password(
    State(services): State<Arc<AppServices>>,
    _user: User,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, ApplicationError> {
    services
        .auth
        .change_password(payload.current_password, payload.new_password)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use crate::application::{composition::MockAppServicesBuilder, errors::DataError};

    use super::*;

    mod sign_up {
        use super::*;

        #[tokio::test]
        async fn returns_the_issued_token() {
            let mut builder = MockAppServicesBuilder::new();
            builder
                .auth
                .expect_sign_up()
                .withf(|password| password == "secret")
                .times(1)
                .returning(|_| Ok("token".to_string()));

            let result = sign_up(
                State(Arc::new(builder.build())),
                Json(SignUpRequest {
                    password: "secret".to_string(),
                }),
            )
            .await;

            let Json(response) = result.unwrap();
            assert_eq!(response.token, "token");
        }
    }

    mod sign_in {
        use super::*;

        #[tokio::test]
        async fn propagates_service_errors() {
            let mut builder = MockAppServicesBuilder::new();
            builder
                .auth
                .expect_sign_in()
                .times(1)
                .returning(|_| Err(DataError::NotFound("missing".to_string()).into()));

            let result = sign_in(
                State(Arc::new(builder.build())),
                Json(SignInRequest {
                    password: "secret".to_string(),
                }),
            )
            .await;

            assert!(matches!(result, Err(ApplicationError::Data(DataError::NotFound(_)))));
        }
    }

    mod change_password {
        use super::*;

        #[tokio::test]
        async fn returns_no_content_when_password_changed() {
            let mut builder = MockAppServicesBuilder::new();
            builder
                .auth
                .expect_change_password()
                .withf(|current_password, new_password| current_password == "old" && new_password == "new")
                .times(1)
                .returning(|_, _| Ok(()));

            let response = change_password(
                State(Arc::new(builder.build())),
                User::default(),
                Json(ChangePasswordRequest {
                    current_password: "old".to_string(),
                    new_password: "new".to_string(),
                }),
            )
            .await
            .unwrap()
            .into_response();

            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }
    }
}
