use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::application::errors::{ApplicationError, AuthenticationError, LightningError, RGBError};

const STATUS_ERROR: &str = "ERROR";
const INTERNAL_SERVER_ERROR: &str =
    "Internal server error, Please contact your administrator or try later";

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApplicationError::RGB(RGBError::ContractIssuance(msg))
            | ApplicationError::RGB(RGBError::Utxos(msg))
            | ApplicationError::RGB(RGBError::Send(msg)) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            ApplicationError::RGB(RGBError::Invoice(msg)) => (StatusCode::BAD_REQUEST, msg),
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
        let (status, error_message) = match self {
            AuthenticationError::MissingCredentials(msg) => (StatusCode::BAD_REQUEST, msg),
            AuthenticationError::JWT(msg) => (StatusCode::UNAUTHORIZED, msg),
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
