use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use regex::Regex;

use crate::{
    adapters::app::AppState,
    application::{
        dtos::{
            LNUrlpInvoiceQueryParams, LNUrlpInvoiceResponse, LightningAddressResponse,
            LightningPaymentResponse, PaginationQueryParams, RegisterLightningAddressRequest,
            SendPaymentRequest,
        },
        errors::{ApplicationError, DataError},
    },
    domains::{lightning::entities::LNURLPayRequest, users::entities::AuthUser},
};

const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

pub struct LightningAddressHandler;

impl LightningAddressHandler {
    pub fn well_known_routes() -> Router<Arc<AppState>> {
        Router::new().route("/:username", get(Self::well_known_lnurlp))
    }

    pub fn addresses_routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(Self::list))
            .route("/", post(Self::register))
            .route("/pay", post(Self::pay))
            .route("/:username", get(Self::get))
            .route("/:username/invoice", get(Self::invoice))
    }

    async fn well_known_lnurlp(
        Path(username): Path<String>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LNURLPayRequest>, ApplicationError> {
        let lnurlp = app_state.lightning.generate_lnurlp(username).await?;

        Ok(lnurlp.into())
    }

    async fn invoice(
        Path(username): Path<String>,
        Query(query_params): Query<LNUrlpInvoiceQueryParams>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LNUrlpInvoiceResponse>, ApplicationError> {
        let invoice = app_state
            .lightning
            .generate_invoice(
                username,
                query_params.amount,
                query_params.comment.unwrap_or_default(),
            )
            .await?;

        Ok(LNUrlpInvoiceResponse::new(invoice.bolt11).into())
    }

    async fn register(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        // Length check
        let username_length = payload.username.len();
        if username_length < MIN_USERNAME_LENGTH || username_length > MAX_USERNAME_LENGTH {
            return Err(
                DataError::RequestValidation("Invlaid username length.".to_string()).into(),
            );
        }

        // Regex validation for allowed characters
        let email_username_re = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$").unwrap(); // Can't fail by assertion
        if !email_username_re.is_match(&payload.username) {
            return Err(
                DataError::RequestValidation("Invalid username format.".to_string()).into(),
            );
        }

        let lightning_address = app_state
            .lightning
            .register_lightning_address(user, payload.username)
            .await?;

        Ok(Json(lightning_address.into()))
    }

    async fn get(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(username): Path<String>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        let lightning_address = app_state
            .lightning
            .get_lightning_address(user, username)
            .await?;

        Ok(Json(lightning_address.into()))
    }

    async fn list(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaginationQueryParams>,
    ) -> Result<Json<Vec<LightningAddressResponse>>, ApplicationError> {
        let limit = query_params.limit.unwrap_or(100);
        let offset = query_params.offset.unwrap_or(0);

        let lightning_addresses = app_state
            .lightning
            .list_lightning_addresses(user, limit, offset)
            .await?;

        let response: Vec<LightningAddressResponse> =
            lightning_addresses.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn pay(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<SendPaymentRequest>,
    ) -> Result<Json<LightningPaymentResponse>, ApplicationError> {
        let payment = app_state
            .lightning
            .send_payment(user, payload.input, payload.amount_msat, payload.comment)
            .await?;

        Ok(Json(payment.into()))
    }
}
