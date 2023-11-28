use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};

use crate::{
    adapters::lightning::DynLightningClient,
    application::{
        dtos::{LightningInvoiceQueryParams, LightningInvoiceResponse, LightningWellKnownResponse},
        errors::ApplicationError,
    },
};

const MAX_SENDABLE: u64 = 1000000000;
const MIN_SENDABLE: u64 = 1000;
const MAX_COMMENT_CHARS: u8 = 0;
const LNURL_TYPE: &str = "payRequest";
const DOMAIN: &str = "localhost:3000";

pub struct LightningHandler;

impl LightningHandler {
    pub fn well_known_routes() -> Router {
        Router::new().route("/lnurlp/:username", get(well_known_lnurlp))
    }

    pub fn routes(lightning_client: DynLightningClient) -> Router {
        Router::new()
            .route("/:username/invoice", get(invoice))
            .with_state(lightning_client)
    }
}

async fn well_known_lnurlp(
    Path(username): Path<String>,
) -> Result<Json<LightningWellKnownResponse>, ApplicationError> {
    println!(
        "Generating lightning well-known JSON response for {}",
        username
    );

    let metadata = [
        [
            "text/plain".to_string(),
            "Numeraire SwissKnife implementation".to_string(),
        ],
        [
            "text/identifier".to_string(),
            format!("{}@{}", username, DOMAIN),
        ],
    ];

    let response = LightningWellKnownResponse {
        callback: format!("{}/lightning/invoice/{}", DOMAIN, username),
        max_sendable: MAX_SENDABLE,
        min_sendable: MIN_SENDABLE,
        metadata: serde_json::to_string(&metadata).unwrap(),
        comment_allowed: Some(MAX_COMMENT_CHARS),
        withdraw_link: None,
        tag: LNURL_TYPE.to_string(),
    };

    Ok(response.into())
}

async fn invoice(
    Path(username): Path<String>,
    Query(query_params): Query<LightningInvoiceQueryParams>,
    State(lightning_client): State<DynLightningClient>,
) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
    println!("Generating invoice for {}", username);

    let invoice = match lightning_client.invoice(query_params.amount).await {
        Ok(invoice) => invoice,
        Err(e) => {
            eprintln!("Error generating invoice: {:?}", e);
            return Err(e);
        }
    };

    let response = LightningInvoiceResponse {
        pr: "".to_string(),
        success_action: None,
        disposable: None,
        routes: vec![],
    };

    Ok(response.into())
}
