use axum::{
    http::{header::WWW_AUTHENTICATE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use tracing::{error, warn};

use crate::application::errors::{ApplicationError, AuthenticationError, AuthorizationError};

const INTERNAL_SERVER_ERROR_MSG: &str =
    "Internal server error, Please contact your administrator or try later";

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        match self {
            ApplicationError::Authentication(error) => error.into_response(),
            ApplicationError::Authorization(error) => error.into_response(),
            _ => {
                error!("{}", self.to_string());

                let status = StatusCode::INTERNAL_SERVER_ERROR;
                let body = generate_body(status, INTERNAL_SERVER_ERROR_MSG);
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
            _ => "Access denied",
        };

        warn!("{}", self.to_string());

        let status = StatusCode::FORBIDDEN;
        let body = generate_body(status, error_message);
        (status, body).into_response()
    }
}

impl IntoResponse for AuthenticationError {
    fn into_response(self) -> Response {
        let (error_message, header_message) = match self {
            AuthenticationError::MissingBearerToken(_) => (
                "Missing authentication token",
                "Bearer realm=\"swissknife\", error=\"invalid_request\"",
            ),
            AuthenticationError::JWT(_) => (
                "Invalid authentication token",
                "Bearer realm=\"swissknife\", error=\"invalid_token\"",
            ),
            _ => (
                "Failed authentication",
                "Bearer realm=\"swissknife\", error=\"failed_authentication\"",
            ),
        };

        warn!("{}", self.to_string());

        let status = StatusCode::UNAUTHORIZED;
        let body = generate_body(status, error_message);
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

fn generate_body(status: StatusCode, reason: &str) -> Json<Value> {
    Json(json!({
        "status": status.as_str(),
        "reason": reason,
    }))
}
