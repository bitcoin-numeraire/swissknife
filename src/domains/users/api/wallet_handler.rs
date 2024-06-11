use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    application::{
        dtos::{NewInvoiceRequest, RegisterLightningAddressRequest, SendPaymentRequest},
        errors::{ApplicationError, DataError},
    },
    domains::{
        invoices::entities::{Invoice, InvoiceFilter, InvoiceStatus},
        lightning::entities::{LnAddress, LnAddressFilter},
        payments::entities::{Payment, PaymentFilter, PaymentStatus},
        users::entities::{AuthUser, UserBalance, Wallet},
    },
    infra::app::AppState,
};

pub struct WalletHandler;

impl WalletHandler {
    pub fn routes() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(Self::get_wallet))
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

    async fn get_wallet(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<Wallet>, ApplicationError> {
        let wallet = app_state.services.wallet.get(user.sub).await?;
        Ok(Json(wallet.into()))
    }

    async fn pay(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(mut payload): Json<SendPaymentRequest>,
    ) -> Result<Json<Payment>, ApplicationError> {
        payload.user_id = Some(user.sub);
        let payment = app_state.services.payment.pay(payload).await?;
        Ok(Json(payment.into()))
    }

    async fn get_balance(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
    ) -> Result<Json<UserBalance>, ApplicationError> {
        let balance = app_state.services.wallet.get_balance(user.sub).await?;
        Ok(balance.into())
    }

    async fn new_invoice(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<NewInvoiceRequest>,
    ) -> Result<Json<Invoice>, ApplicationError> {
        let invoice = app_state
            .services
            .invoice
            .invoice(
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
    ) -> Result<Json<LnAddress>, ApplicationError> {
        let ln_addresses = app_state
            .services
            .lnurl
            .list(LnAddressFilter {
                user_id: Some(user.sub),
                ..Default::default()
            })
            .await?;

        let ln_address = ln_addresses
            .first()
            .cloned()
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        Ok(Json(ln_address.into()))
    }

    async fn register_address(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Json(payload): Json<RegisterLightningAddressRequest>,
    ) -> Result<Json<LnAddress>, ApplicationError> {
        let ln_address = app_state
            .services
            .lnurl
            .register(user.sub, payload.username)
            .await?;
        Ok(Json(ln_address.into()))
    }

    async fn list_payments(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Query(mut query_params): Query<PaymentFilter>,
    ) -> Result<Json<Vec<Payment>>, ApplicationError> {
        query_params.user_id = Some(user.sub);
        let payments = app_state.services.payment.list(query_params).await?;

        let response: Vec<Payment> = payments.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn get_payment(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<Payment>, ApplicationError> {
        let payments = app_state
            .services
            .payment
            .list(PaymentFilter {
                user_id: Some(user.sub),
                ids: Some(vec![id]),
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
        Query(mut query_params): Query<InvoiceFilter>,
    ) -> Result<Json<Vec<Invoice>>, ApplicationError> {
        query_params.user_id = Some(user.sub);
        let invoices = app_state.services.invoice.list(query_params).await?;

        let response: Vec<Invoice> = invoices.into_iter().map(Into::into).collect();

        Ok(response.into())
    }

    async fn get_invoice(
        State(app_state): State<Arc<AppState>>,
        user: AuthUser,
        Path(id): Path<Uuid>,
    ) -> Result<Json<Invoice>, ApplicationError> {
        let invoices = app_state
            .services
            .invoice
            .list(InvoiceFilter {
                user_id: Some(user.sub),
                ids: Some(vec![id]),
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
            .services
            .invoice
            .delete_many(InvoiceFilter {
                user_id: Some(user.sub),
                status: Some(InvoiceStatus::EXPIRED),
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
            .services
            .payment
            .delete_many(PaymentFilter {
                user_id: Some(user.sub),
                status: Some(PaymentStatus::FAILED),
                ..Default::default()
            })
            .await?;
        Ok(n_deleted.into())
    }
}
