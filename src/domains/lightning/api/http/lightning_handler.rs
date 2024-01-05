use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use breez_sdk_core::{NodeState, Payment};
use tracing::{debug, error};

use crate::{
    adapters::{auth::Authenticator, database::DatabaseClient, lightning::LightningClient},
    application::{
        dtos::{
            LightningAddressResponse, LightningInvoiceQueryParams, LightningInvoiceResponse,
            LightningWellKnownResponse, RegisterLightningAddressRequest, SuccessAction,
        },
        errors::{ApplicationError, DatabaseError},
    },
    domains::lightning::entities::LightningAddress,
    domains::users::entities::AuthUser,
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

    pub fn routes(
        lightning_client: Arc<dyn LightningClient>,
    ) -> Router<Option<Arc<dyn Authenticator>>> {
        Router::new()
            .route("/lnurlp/:username/callback", get(Self::invoice))
            .route("/node-info", get(Self::node_info))
            .route("/list-payments", get(Self::list_payments))
            .with_state(lightning_client)
        /* .route(
            "/lightning_addresses",
            post(Self::register_lightning_address),
        )*/
    }

    async fn well_known_lnurlp(
        Path(username): Path<String>,
    ) -> Result<Json<LightningWellKnownResponse>, ApplicationError> {
        debug!(
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
        State(lightning_client): State<Arc<dyn LightningClient>>,
    ) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
        debug!("Generating invoice for {}", username);

        let invoice = match lightning_client
            .invoice(query_params.amount, generate_metadata(username))
            .await
        {
            Ok(invoice) => invoice,
            Err(e) => {
                error!(error = ?e, "Error generating invoice");
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
        user: AuthUser,
        State(lightning_client): State<Arc<dyn LightningClient>>,
    ) -> Result<Json<NodeState>, ApplicationError> {
        debug!(user = ?user, "Getting node info");

        let node_info = match lightning_client.node_info().await {
            Ok(node_info) => node_info,
            Err(e) => {
                error!(error = ?e, "Error getting node info");
                return Err(e.into());
            }
        };

        Ok(node_info.into())
    }

    async fn list_payments(
        State(lightning_client): State<Arc<dyn LightningClient>>,
    ) -> Result<Json<Vec<Payment>>, ApplicationError> {
        debug!("Listing payments");

        let payments = match lightning_client.list_payments().await {
            Ok(payments) => payments,
            Err(e) => {
                error!(error = ?e, "Error listing payments");
                return Err(e.into());
            }
        };

        Ok(payments.into())
    }

    /*async fn register_lightning_address(
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
        Extension(db_client): Extension<Arc<dyn DatabaseClient>>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        println!("Registering lightning address: {:?}", payload);

        let lightning_address: LightningAddress = sqlx::query_as!(
            LightningAddress,
            // language=PostgreSQL
            r#"
                insert into "lightning_addresses"(user_id, username)
                values ($1, $2)
                returning *
            "#,
            user.sub,
            payload.username
        )
        .fetch_one(&db_client.pool())
        .await
        .map_err(|e| {
            let err_message = "Database error";
            debug!(error = ?e, err_message);
            DatabaseError::Query(e.to_string())
        })?;

        let response = LightningAddressResponse {
            id: lightning_address.id,
            user_id: lightning_address.user_id,
            username: lightning_address.username,
            active: lightning_address.active,
            created_at: lightning_address.created_at,
            updated_at: lightning_address.updated_at,
            deleted_at: lightning_address.deleted_at,
        };

        Ok(response.into())
    }
    */
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
