use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    application::{
        dtos::{
            LightningAddressResponse, LightningInvoiceResponse, LightningPaymentResponse,
            NewInvoiceRequest, PaginationQueryParams, RegisterLightningAddressRequest,
            SendPaymentRequest,
        },
        errors::ApplicationError,
    },
    domains::{lightning::entities::UserBalance, users::entities::AuthUser},
    infra::app::AppState,
};

pub struct WalletHandler;

impl WalletHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/pay", post(Self::pay))
            .route("/balance", get(Self::get_balance))
            .route("/lightning-address", get(Self::get_address))
            .route("/lightning-address", post(Self::register_address))
            .route("/payments", get(Self::list_payments))
            .route("/payments/:id", get(Self::get_payment))
            .route("/invoices", get(Self::list_invoices))
            .route("/invoices/:id", get(Self::get_invoice))
            .route("/invoices", post(Self::new_invoice))
            .route("/invoices", delete(Self::delete_expired_invoices))
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
        let balance = app_state.wallet.get_balance(user).await?;

        Ok(balance.into())
    }

    async fn new_invoice(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<NewInvoiceRequest>,
    ) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
        let invoice = app_state
            .wallet
            .generate_Lightning_invoice(
                user,
                payload.amount_msat,
                payload.description,
                payload.expiry,
            )
            .await?;

        Ok(Json(invoice.into()))
    }

    async fn get_address(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        let lightning_address = app_state.lightning.get_address_by_user_id(user.sub).await?;
        Ok(Json(lightning_address.into()))
    }

    async fn register_address(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LightningAddressResponse>, ApplicationError> {
        let lightning_address = app_state
            .lightning
            .register_address(user.sub, payload.username)
            .await?;
        Ok(Json(lightning_address.into()))
    }

    async fn list_payments(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaginationQueryParams>,
    ) -> Result<Json<Vec<LightningPaymentResponse>>, ApplicationError> {
        let payments = app_state
            .lightning
            .list_payments(user, query_params.limit, query_params.offset)
            .await?;

        let response: Vec<LightningPaymentResponse> =
            payments.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn get_payment(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<LightningPaymentResponse>, ApplicationError> {
        let payment = app_state.lightning.get_payment(user, id).await?;

        Ok(Json(payment.into()))
    }

    async fn list_invoices(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(query_params): Query<PaginationQueryParams>,
    ) -> Result<Json<Vec<LightningInvoiceResponse>>, ApplicationError> {
        let invoices = app_state
            .wallet
            .list_lightning_invoices(user, query_params.limit, query_params.offset)
            .await?;

        let response: Vec<LightningInvoiceResponse> =
            invoices.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn get_invoice(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
        let payment = app_state.wallet.get_lightning_invoice(user, id).await?;
        Ok(Json(payment.into()))
    }

    async fn delete_expired_invoices(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<u64>, ApplicationError> {
        let n_deleted = app_state.wallet.delete_expired_invoices(user).await?;
        Ok(n_deleted.into())
    }
}
