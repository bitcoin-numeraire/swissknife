use axum::{
    http::{header::WWW_AUTHENTICATE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::application::errors::{AuthenticationError, LightningError, RGBError};

const STATUS_ERROR: &str = "ERROR";
const INTERNAL_SERVER_ERROR: &str =
    "Internal server error, Please contact your administrator or try later";

impl IntoResponse for RGBError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            RGBError::ContractIssuance(msg)
            | RGBError::CreateUtxos(msg)
            | RGBError::Send(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            RGBError::Invoice(msg) => (StatusCode::BAD_REQUEST, msg),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                INTERNAL_SERVER_ERROR.to_string(),
            ),
        };

        let body = Json(json!({
            "status": STATUS_ERROR.to_string(),
            "reason": error_message,
        }));

        (status, body).into_response()
    }
}

impl IntoResponse for LightningError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            LightningError::Invoice(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                INTERNAL_SERVER_ERROR.to_string(),
            ),
        };

        let body = Json(json!({
            "status": STATUS_ERROR.to_string(),
            "reason": error_message,
        }));

        (status, body).into_response()
    }
}

impl IntoResponse for AuthenticationError {
    fn into_response(self) -> Response {
        let (status, error_message, header_message) = match self {
            AuthenticationError::MissingBearerToken(msg)  => 
                (StatusCode::UNAUTHORIZED, msg.clone(), format!("Bearer realm=\"swissknife\", error=\"invalid_request\", error_description=\"{}\"", msg)),
            AuthenticationError::JWT(msg) => 
                (StatusCode::UNAUTHORIZED, msg.clone(), format!("Bearer realm=\"swissknife\", error=\"invalid_token\", error_description=\"{}\"", msg)),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                INTERNAL_SERVER_ERROR.to_string(),
                "".to_string()
            ),
        };

        let body = Json(json!({
            "status": status.canonical_reason(),
            "reason": error_message,
        }));
        
        let mut response = (status, body).into_response();

        response.headers_mut().insert(
            WWW_AUTHENTICATE,
            HeaderValue::from_str(&header_message).unwrap(),
        );

        response
    }
}
