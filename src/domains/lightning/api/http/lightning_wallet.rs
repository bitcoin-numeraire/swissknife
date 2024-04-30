use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use crate::{
    application::{
        dtos::{
            LNUrlpInvoiceQueryParams, LightningAddressResponse, LightningInvoiceResponse,
            LightningPaymentResponse, PaginationQueryParams, SendPaymentRequest,
        },
        errors::ApplicationError,
    },
    domains::{lightning::entities::UserBalance, users::entities::AuthUser},
    infra::app::AppState,
};

pub struct LightningWalletHandler;

impl LightningWalletHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/pay", post(Self::pay))
            .route("/balance", get(Self::get_balance))
            .route("/new-invoice", get(Self::new_invoice))
            .route("/addresses", get(Self::list_addresses))
            .route("/payments", get(Self::list_payments))
            .route("/invoices", get(Self::list_invoices))
    }

    async fn pay(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<SendPaymentRequest>,
    ) -> Result<Json<LightningPaymentResponse>, ApplicationError> {
        let payment = app_state
            .lightning
            .pay(user, payload.input, payload.amount_msat, payload.comment)
            .await?;

        Ok(Json(payment.into()))
    }

    async fn get_balance(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<UserBalance>, ApplicationError> {
        let balance = app_state.lightning.get_balance(user).await?;

        Ok(balance.into())
    }

    async fn new_invoice(
        Path(username): Path<String>,
        Query(query_params): Query<LNUrlpInvoiceQueryParams>,
        State(app_state): State<Arc<AppState>>,
    ) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
        let invoice = app_state
            .lightning
            .generate_invoice(
                username,
                query_params.amount,
                query_params.comment.unwrap_or_default(),
            )
            .await?;

        Ok(Json(invoice.into()))
    }

    async fn list_addresses(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Vec<LightningAddressResponse>>, ApplicationError> {
        let lightning_addresses = app_state
            .lightning
            .list_lightning_addresses(user, None, None)
            .await?;

        let response: Vec<LightningAddressResponse> =
            lightning_addresses.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn list_payments(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaginationQueryParams>,
    ) -> Result<Json<Vec<LightningPaymentResponse>>, ApplicationError> {
        let payments = app_state
            .lightning
            .list_lightning_payments(user, query_params.limit, query_params.offset)
            .await?;

        let response: Vec<LightningPaymentResponse> =
            payments.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn list_invoices(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaginationQueryParams>,
    ) -> Result<Json<Vec<LightningInvoiceResponse>>, ApplicationError> {
        let invoices = app_state
            .lightning
            .list_lightning_invoices(user, query_params.limit, query_params.offset)
            .await?;

        let response: Vec<LightningInvoiceResponse> =
            invoices.into_iter().map(Into::into).collect();

        Ok(response.into())
    }
}
