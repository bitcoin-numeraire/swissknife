use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::application::errors::ApplicationError;

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            // ApplicationError::Wallet => (StatusCode::NOT_FOUND, "User not found"),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error, Please contact your administrator or try later",
            ),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
