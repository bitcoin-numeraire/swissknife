use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use breez_sdk_core::{NodeState, Payment};

use crate::{
    adapters::lightning::DynLightningClient,
    application::{
        dtos::{
            LightningInvoiceQueryParams, LightningInvoiceResponse, LightningWellKnownResponse,
            SuccessAction,
        },
        errors::ApplicationError,
    },
};

const MAX_SENDABLE: u64 = 1000000000;
const MIN_SENDABLE: u64 = 1000;
const MAX_COMMENT_CHARS: u8 = 255;
const LNURL_TYPE: &str = "payRequest";
const DOMAIN: &str = "numerairelocal.tech";

pub struct LightningHandler;

impl LightningHandler {
    pub fn well_known_routes() -> Router {
        Router::new().route("/lnurlp/:username", get(Self::well_known_lnurlp))
    }

    pub fn routes(lightning_client: DynLightningClient) -> Router {
        Router::new()
            .route("/lnurlp/:username/callback", get(Self::invoice))
            .route("/node-info", get(Self::node_info))
            .route("/list-payments", get(Self::list_payments))
            .with_state(lightning_client)
    }

    async fn well_known_lnurlp(
        Path(username): Path<String>,
    ) -> Result<Json<LightningWellKnownResponse>, ApplicationError> {
        println!(
            "Generating lightning well-known JSON response for {}",
            username
        );

        let response = LightningWellKnownResponse {
            callback: format!("https://{}/lightning/lnurlp/{}/callback", DOMAIN, username),
            max_sendable: MAX_SENDABLE,
            min_sendable: MIN_SENDABLE,
            metadata: generate_metadata(username),
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

        let invoice = match lightning_client
            .invoice(query_params.amount, generate_metadata(username))
            .await
        {
            Ok(invoice) => invoice,
            Err(e) => {
                eprintln!("Error generating invoice: {:?}", e);
                return Err(e.into());
            }
        };

        let response = LightningInvoiceResponse {
            pr: invoice,
            success_action: Some(SuccessAction {
                tag: "message".to_string(),
                message: Some("Thanks for the sats!".to_string()),
            }),
            disposable: None,
            routes: vec![],
        };

        Ok(response.into())
    }

    async fn node_info(
        State(lightning_client): State<DynLightningClient>,
    ) -> Result<Json<NodeState>, ApplicationError> {
        println!("Getting node info");

        let node_info = match lightning_client.node_info().await {
            Ok(node_info) => node_info,
            Err(e) => {
                eprintln!("Error getting node info: {:?}", e);
                return Err(e.into());
            }
        };

        Ok(node_info.into())
    }

    async fn list_payments(
        State(lightning_client): State<DynLightningClient>,
    ) -> Result<Json<Vec<Payment>>, ApplicationError> {
        println!("Listing payments");

        let payments = match lightning_client.list_payments().await {
            Ok(payments) => payments,
            Err(e) => {
                eprintln!("Error listing payments: {:?}", e);
                return Err(e.into());
            }
        };

        Ok(payments.into())
    }
}

fn generate_metadata(username: String) -> String {
    let metadata = [
        [
            "text/plain".to_string(),
            format!("{} never refuses sats", username),
        ],
        [
            "text/identifier".to_string(),
            format!("{}@{}", username, DOMAIN),
        ],
    ];

    serde_json::to_string(&metadata).unwrap()
}