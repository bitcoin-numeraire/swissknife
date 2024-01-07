use axum::{
    http::{header::WWW_AUTHENTICATE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use tracing::{error, warn};

use crate::application::errors::{AuthenticationError, LightningError, RGBError};

const INTERNAL_SERVER_ERROR: &str =
    "Internal server error, Please contact your administrator or try later";

// TODO: Match errors with appropriate status codes when use cases are implemented
impl IntoResponse for RGBError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            _ => (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_SERVER_ERROR),
        };

        error!("{}", self.to_string());

        let body = generate_body(status, error_message);
        (status, body).into_response()
    }
}

impl IntoResponse for LightningError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            _ => (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_SERVER_ERROR),
        };

        error!("{}", self.to_string());

        let body = generate_body(status, error_message);
        (status, body).into_response()
    }
}

impl IntoResponse for AuthenticationError {
    fn into_response(self) -> Response {
        let (status, error_message, header_message) = match self {
            AuthenticationError::MissingBearerToken(_) => (
                StatusCode::UNAUTHORIZED,
                "Missing authentication token",
                "Bearer realm=\"swissknife\", error=\"invalid_request\"",
            ),
            AuthenticationError::JWT(_) => (
                StatusCode::UNAUTHORIZED,
                "Invalid authentication token",
                "Bearer realm=\"swissknife\", error=\"invalid_token\"",
            ),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_SERVER_ERROR, ""),
        };

        // Log the error with appropriate level and message
        match status {
            StatusCode::UNAUTHORIZED => warn!("{}", self.to_string()),
            _ => error!("{}", self.to_string()),
        }

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

fn generate_body(status: StatusCode, error_message: &str) -> Json<Value> {
    Json(json!({
        "status": status.as_str(),
        "reason": error_message,
    }))
}
