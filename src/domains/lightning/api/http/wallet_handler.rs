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
            LightningAddressFilter, LightningAddressResponse, LightningInvoiceFilter,
            LightningInvoiceResponse, LightningPaymentFilter, LightningPaymentResponse,
            NewInvoiceRequest, RegisterLightningAddressRequest, SendPaymentRequest,
        },
        errors::{ApplicationError, DataError},
    },
    domains::{
        lightning::entities::{LightningInvoiceStatus, LightningPaymentStatus, UserBalance},
        users::entities::AuthUser,
    },
    infra::app::AppState,
};

pub struct WalletHandler;

impl WalletHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/balance", get(Self::get_balance))
            .route("/lightning-address", get(Self::get_address))
            .route("/lightning-address", post(Self::register_address))
            .route("/payments", post(Self::pay))
            .route("/payments", get(Self::list_payments))
            .route("/payments/:id", get(Self::get_payment))
            .route("/payments", delete(Self::delete_failed_payments))
            .route("/invoices", get(Self::list_invoices))
            .route("/invoices/:id", get(Self::get_invoice))
            .route("/invoices", post(Self::new_invoice))
            .route("/invoices", delete(Self::delete_expired_invoices))
    }

    async fn pay(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(mut payload): Json<SendPaymentRequest>,
    ) -> Result<Json<LightningPaymentResponse>, ApplicationError> {
        payload.user_id = user.sub;
        let payment = app_state.lightning.pay(payload).await?;
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
            .lightning
            .generate_invoice(
                user.sub,
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
        let lightning_addresses = app_state
            .lightning
            .list_addresses(LightningAddressFilter {
                user_id: Some(user.sub),
                ..Default::default()
            })
            .await?;

        let lightning_address = lightning_addresses
            .first()
            .cloned()
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

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
        Query(mut query_params): Query<LightningPaymentFilter>,
    ) -> Result<Json<Vec<LightningPaymentResponse>>, ApplicationError> {
        query_params.user_id = Some(user.sub);
        let payments = app_state.lightning.list_payments(query_params).await?;

        let response: Vec<LightningPaymentResponse> =
            payments.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn get_payment(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<LightningPaymentResponse>, ApplicationError> {
        let payments = app_state
            .lightning
            .list_payments(LightningPaymentFilter {
                user_id: Some(user.sub),
                id: Some(id),
                ..Default::default()
            })
            .await?;

        let lightning_payment = payments
            .first()
            .cloned()
            .ok_or_else(|| DataError::NotFound("Lightning payment not found.".to_string()))?;

        Ok(Json(lightning_payment.into()))
    }

    async fn list_invoices(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(mut query_params): Query<LightningInvoiceFilter>,
    ) -> Result<Json<Vec<LightningInvoiceResponse>>, ApplicationError> {
        query_params.user_id = Some(user.sub);
        let invoices = app_state.lightning.list_invoices(query_params).await?;

        let response: Vec<LightningInvoiceResponse> =
            invoices.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn get_invoice(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<LightningInvoiceResponse>, ApplicationError> {
        let invoices = app_state
            .lightning
            .list_invoices(LightningInvoiceFilter {
                user_id: Some(user.sub),
                id: Some(id),
                ..Default::default()
            })
            .await?;

        let lightning_invoice = invoices
            .first()
            .cloned()
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        Ok(Json(lightning_invoice.into()))
    }

    async fn delete_expired_invoices(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<u64>, ApplicationError> {
        let n_deleted = app_state
            .lightning
            .delete_invoices(LightningInvoiceFilter {
                user_id: Some(user.sub),
                status: Some(LightningInvoiceStatus::EXPIRED),
                ..Default::default()
            })
            .await?;
        Ok(n_deleted.into())
    }

    async fn delete_failed_payments(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<u64>, ApplicationError> {
        let n_deleted = app_state
            .lightning
            .delete_payments(LightningPaymentFilter {
                user_id: Some(user.sub),
                status: Some(LightningPaymentStatus::FAILED),
                ..Default::default()
            })
            .await?;
        Ok(n_deleted.into())
    }
}
