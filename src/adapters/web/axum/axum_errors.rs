use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::application::errors::{ApplicationError, RGBError};

const STATUS_ERROR: String = "ERROR".to_string();

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApplicationError::RGB(RGBError::ContractIssuance(msg))
            | ApplicationError::RGB(RGBError::Utxos(msg))
            | ApplicationError::RGB(RGBError::Send(msg)) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            ApplicationError::RGB(RGBError::Invoice(msg)) => (StatusCode::BAD_REQUEST, msg),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error, Please contact your administrator or try later".to_string(),
            ),
        };

        let body = Json(json!({
            "status": STATUS_ERROR,
            "reason": error_message,
        }));

        (status, body).into_response()
    }
}
