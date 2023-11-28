use axum::{extract::Path, routing::get, Json, Router};

use crate::application::{dtos::LightningInvoiceResponse, errors::ApplicationError};

const MAX_SENDABLE: u64 = 1000000000;
const MIN_SENDABLE: u64 = 1000;
const MAX_COMMENT_CHARS: u8 = 0;
const LNURL_TYPE: &str = "payRequest";
const DOMAIN: &str = "localhost:3000";

pub struct LightningHandler;

impl LightningHandler {
    pub fn well_known_routes() -> Router {
        Router::new().route("lnurlp/:username", get(well_known_lnurlp))
    }
}

async fn well_known_lnurlp(
    Path(username): Path<String>,
) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
    println!(
        "Generating lightning well-known JSON response for {}",
        username
    );

    let metadata = [
        ["text/plain".to_string(), "".to_string()],
        [
            "text/identifier".to_string(),
            format!("{}@{}", username, DOMAIN),
        ],
    ];

    let response = LightningInvoiceResponse {
        callback: format!("{}/lightning/pay", DOMAIN),
        max_sendable: MAX_SENDABLE,
        min_sendable: MIN_SENDABLE,
        metadata: serde_json::to_string(&metadata).unwrap(),
        comment_allowed: Some(MAX_COMMENT_CHARS),
        withdraw_link: None,
        tag: LNURL_TYPE.to_string(),
    };

    Ok(response.into())
}
