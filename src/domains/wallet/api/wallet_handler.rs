use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use utoipa::OpenApi;
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
        users::entities::AuthUser,
        wallet::entities::{UserBalance, Wallet},
    },
    infra::app::AppState,
};

#[derive(OpenApi)]
#[openapi(paths(get_wallet), components(schemas(Wallet)))]
pub struct WalletHandler;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_wallet))
        .route("/balance", get(get_balance))
        .route("/lightning-address", get(get_address))
        .route("/lightning-address", post(register_address))
        .route("/payments", post(pay))
        .route("/payments", get(list_payments))
        .route("/payments/:id", get(get_payment))
        .route("/payments", delete(delete_failed_payments))
        .route("/invoices", get(list_invoices))
        .route("/invoices/:id", get(get_invoice))
        .route("/invoices", post(new_invoice))
        .route("/invoices", delete(delete_expired_invoices))
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Get user wallet")
    )
)]
async fn get_wallet(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<Wallet>, ApplicationError> {
    let wallet = app_state.services.wallet.get(user.sub).await?;
    Ok(wallet.into())
}

async fn pay(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Json(mut payload): Json<SendPaymentRequest>,
) -> Result<Json<Payment>, ApplicationError> {
    payload.user_id = Some(user.sub);
    let payment = app_state.services.payment.pay(payload).await?;
    Ok(payment.into())
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

    Ok(invoice.into())
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

    Ok(ln_address.into())
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
    Ok(ln_address.into())
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

    Ok(lightning_payment.into())
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

    Ok(lightning_invoice.into())
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
            status: Some(InvoiceStatus::Expired),
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
            status: Some(PaymentStatus::Failed),
            ..Default::default()
        })
        .await?;
    Ok(n_deleted.into())
}
