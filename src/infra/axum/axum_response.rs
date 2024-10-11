use axum::{
    http::{header::WWW_AUTHENTICATE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use tracing::{debug, error, trace, warn};

use crate::application::{
    dtos::ErrorResponse,
    errors::{
        ApplicationError, AuthenticationError, AuthorizationError, DataError, LightningError,
    },
};

const INTERNAL_SERVER_ERROR_MSG: &str =
    "Internal server error, Please contact your administrator or try later";

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        match self {
            ApplicationError::Authentication(error) => error.into_response(),
            ApplicationError::Authorization(error) => error.into_response(),
            ApplicationError::Data(error) => error.into_response(),
            ApplicationError::Lightning(error) => error.into_response(),
            _ => {
                error!("{}", self);

                let status = StatusCode::INTERNAL_SERVER_ERROR;
                let body = generate_body(status, INTERNAL_SERVER_ERROR_MSG.to_string());
                (status, body).into_response()
            } // Add additional cases as needed
        }
    }
}

impl IntoResponse for AuthorizationError {
    fn into_response(self) -> Response {
        let error_message = match self {
            AuthorizationError::MissingPermission(_) => {
                "Access denied due to insufficient permissions"
            }
        };

        warn!("{}", self);

        let status = StatusCode::FORBIDDEN;
        let body = generate_body(status, error_message.to_string());
        (status, body).into_response()
    }
}

impl IntoResponse for AuthenticationError {
    fn into_response(self) -> Response {
        let (error_message, header_message) = match self {
            AuthenticationError::InvalidCredentials => ("Invalid credentials", ""),
            AuthenticationError::MissingAuthorizationHeader => (
                "Missing authentication token",
                "Bearer realm=\"swissknife\", error=\"invalid_request\"",
            ),
            AuthenticationError::DecodeJWT(_)
            | AuthenticationError::DecodeJWTHeader(_)
            | AuthenticationError::DecodeJWTKey(_)
            | AuthenticationError::MissingJWTKid
            | AuthenticationError::MissingJWK => (
                "Invalid authentication token",
                "Bearer realm=\"swissknife\", error=\"invalid_token\"",
            ),
            _ => (
                "Failed authentication",
                "Bearer realm=\"swissknife\", error=\"failed_authentication\"",
            ),
        };

        warn!("{}", self);

        let status = StatusCode::UNAUTHORIZED;
        let body = generate_body(status, error_message.to_string());
        let mut response = (status, body).into_response();

        // Add WWW-Authenticate header if needed
        if !header_message.is_empty() {
            if let Ok(header_value) = HeaderValue::from_str(header_message) {
                response
                    .headers_mut()
                    .insert(WWW_AUTHENTICATE, header_value);
            } else {
                error!("Failed to create WWW-Authenticate header");
            }
        }

        response
    }
}

impl IntoResponse for DataError {
    fn into_response(self) -> Response {
        let (error_message, status) = match self {
            DataError::NotFound(_) => {
                debug!("{}", self);
                (self.to_string(), StatusCode::NOT_FOUND)
            }
            DataError::Conflict(_) => {
                warn!("{}", self);
                (self.to_string(), StatusCode::CONFLICT)
            }
            DataError::Validation(_) | DataError::InsufficientFunds(_) => {
                warn!("{}", self);
                (self.to_string(), StatusCode::UNPROCESSABLE_ENTITY)
            }
            DataError::Inconsistency(_) => {
                error!("{}", self);
                (self.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
            }
            DataError::Malformed(_) => {
                trace!("{}", self);
                (self.to_string(), StatusCode::BAD_REQUEST)
            }
        };

        let body = generate_body(status, error_message);
        (status, body).into_response()
    }
}

impl IntoResponse for LightningError {
    fn into_response(self) -> Response {
        let (error_message, status) = match self {
            LightningError::Pay(_)
            | LightningError::Invoice(_)
            | LightningError::ConnectLSP(_)
            | LightningError::SignMessage(_)
            | LightningError::CheckMessage(_)
            | LightningError::RedeemOnChain(_) => {
                warn!("{}", self);
                (self.to_string(), StatusCode::UNPROCESSABLE_ENTITY)
            }
            _ => {
                error!("{}", self);
                (
                    INTERNAL_SERVER_ERROR_MSG.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        };

        let body = generate_body(status, error_message);
        (status, body).into_response()
    }
}

fn generate_body(status: StatusCode, reason: String) -> Json<ErrorResponse> {
    ErrorResponse {
        status: status.to_string(),
        reason,
    }
    .into()
}
