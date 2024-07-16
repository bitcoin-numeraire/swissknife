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
        docs::{
            BAD_REQUEST_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNPROCESSABLE_EXAMPLE,
        },
        dtos::{
            InvoiceResponse, NewInvoiceRequest, PaymentResponse, RegisterLnAddressRequest,
            SendPaymentRequest, WalletResponse,
        },
        errors::{ApplicationError, DataError},
    },
    domains::{
        invoice::{InvoiceFilter, InvoiceStatus},
        ln_address::{LnAddress, LnAddressFilter},
        payment::{PaymentFilter, PaymentStatus},
        user::AuthUser,
    },
    infra::app::AppState,
};

use super::{Contact, UserBalance};

#[derive(OpenApi)]
#[openapi(
    paths(get_wallet, get_wallet_balance, get_wallet_address, register_wallet_address, wallet_pay, list_wallet_payments,
        get_wallet_payment, delete_failed_payments, list_wallet_invoices, get_wallet_invoice, new_wallet_invoice, delete_expired_invoices,
        list_contacts
    ),
    components(schemas(WalletResponse, UserBalance, Contact)),
    tags(
        (name = "Wallet", description = "Wallet endpoints. Available to any authenticated user.")
    ),
)]
pub struct WalletHandler;
pub const CONTEXT_PATH: &str = "/api/wallet";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_wallet))
        .route("/balance", get(get_wallet_balance))
        .route("/lightning-address", get(get_wallet_address))
        .route("/lightning-address", post(register_wallet_address))
        .route("/payments", post(wallet_pay))
        .route("/payments", get(list_wallet_payments))
        .route("/payments/:id", get(get_wallet_payment))
        .route("/payments", delete(delete_failed_payments))
        .route("/invoices", get(list_wallet_invoices))
        .route("/invoices/:id", get(get_wallet_invoice))
        .route("/invoices", post(new_wallet_invoice))
        .route("/contacts", get(list_contacts))
        .route("/invoices", delete(delete_expired_invoices))
}

/// Gets the user wallet
///
/// Returns the user wallet.
#[utoipa::path(
    get,
    path = "",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = WalletResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<WalletResponse>, ApplicationError> {
    let wallet = app_state.services.wallet.get(user.sub).await?;
    Ok(Json(wallet.into()))
}

/// Send a payment
///
/// Pay for a LN invoice, LNURL, LN Address, On-chain or internally to an other user on the same instance. Returns the payment details.
#[utoipa::path(
    post,
    path = "/payments",
    tag = "Payments",
    context_path = CONTEXT_PATH,
    request_body = SendPaymentRequest,
    responses(
        (status = 200, description = "Payment Sent", body = PaymentResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn wallet_pay(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Json(mut payload): Json<SendPaymentRequest>,
) -> Result<Json<PaymentResponse>, ApplicationError> {
    payload.user_id = Some(user.sub);
    let payment = app_state.services.payment.pay(payload).await?;
    Ok(Json(payment.into()))
}

/// Gets the user balance
///
/// Returns the user balance.
#[utoipa::path(
    get,
    path = "/balance",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = UserBalance),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_balance(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<UserBalance>, ApplicationError> {
    let balance = app_state.services.wallet.get_balance(user.sub).await?;
    Ok(balance.into())
}

/// Generate a new invoice
///
/// Returns the generated invoice
#[utoipa::path(
    post,
    path = "/invoices",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    request_body = NewInvoiceRequest,
    responses(
        (status = 200, description = "Invoice Created", body = InvoiceResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn new_wallet_invoice(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Json(payload): Json<NewInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, ApplicationError> {
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

/// Get your LN Address
///
/// Returns the registered address
#[utoipa::path(
    get,
    path = "/lightning-address",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = LnAddress),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_address(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<LnAddress>, ApplicationError> {
    let ln_addresses = app_state
        .services
        .ln_address
        .list(LnAddressFilter {
            user_id: Some(user.sub),
            ..Default::default()
        })
        .await?;

    let ln_address = ln_addresses
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("LN Address not found.".to_string()))?;

    Ok(ln_address.into())
}

/// Register a new LN Address
///
/// Registers an address. Returns the address details. LN Addresses are ready to receive funds through the LNURL protocol upon registration.
#[utoipa::path(
    post,
    path = "/lightning-address",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    request_body = RegisterLnAddressRequest,
    responses(
        (status = 200, description = "LN Address Registered", body = LnAddress),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn register_wallet_address(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Json(payload): Json<RegisterLnAddressRequest>,
) -> Result<Json<LnAddress>, ApplicationError> {
    let ln_address = app_state
        .services
        .ln_address
        .register(user.sub, payload.username)
        .await?;
    Ok(ln_address.into())
}

/// List payments
///
/// Returns all the payments given a filter
#[utoipa::path(
    get,
    path = "/payments",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    params(PaymentFilter),
    responses(
        (status = 200, description = "Success", body = Vec<PaymentResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallet_payments(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Query(mut query_params): Query<PaymentFilter>,
) -> Result<Json<Vec<PaymentResponse>>, ApplicationError> {
    query_params.wallet_id = Some(user.sub);
    let payments = app_state.services.payment.list(query_params).await?;

    let response: Vec<PaymentResponse> = payments.into_iter().map(Into::into).collect();

    Ok(response.into())
}

/// Find a payment
///
/// Returns the payment by its ID
#[utoipa::path(
    get,
    path = "/payments/{id}",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = PaymentResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_payment(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PaymentResponse>, ApplicationError> {
    let payments = app_state
        .services
        .payment
        .list(PaymentFilter {
            wallet_id: Some(user.sub),
            ids: Some(vec![id]),
            ..Default::default()
        })
        .await?;

    let payment = payments
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("Payment not found.".to_string()))?;

    Ok(Json(payment.into()))
}

/// List invoices
///
/// Returns all the invoices given a filter
#[utoipa::path(
    get,
    path = "/invoices",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    params(InvoiceFilter),
    responses(
        (status = 200, description = "Success", body = Vec<InvoiceResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallet_invoices(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Query(mut query_params): Query<InvoiceFilter>,
) -> Result<Json<Vec<InvoiceResponse>>, ApplicationError> {
    query_params.wallet_id = Some(user.sub);
    let invoices = app_state.services.invoice.list(query_params).await?;

    let response: Vec<InvoiceResponse> = invoices.into_iter().map(Into::into).collect();

    Ok(response.into())
}

/// Find an invoice
///
/// Returns the invoice by its ID
#[utoipa::path(
    get,
    path = "/invoices/{id}",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = InvoiceResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_invoice(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<InvoiceResponse>, ApplicationError> {
    let invoices = app_state
        .services
        .invoice
        .list(InvoiceFilter {
            wallet_id: Some(user.sub),
            ids: Some(vec![id]),
            ..Default::default()
        })
        .await?;

    let invoice = invoices
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?;

    Ok(Json(invoice.into()))
}

/// List contacts
///
/// Returns all the contacts
#[utoipa::path(
    get,
    path = "/contacts",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = Vec<Contact>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_contacts(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<Vec<Contact>>, ApplicationError> {
    let contacts = app_state.services.wallet.list_contacts(user.sub).await?;
    Ok(contacts.into())
}

/// Delete expired invoices
///
/// Deletes all the invoices with status `Èxpired`. Returns the number of deleted invoices
#[utoipa::path(
    delete,
    path = "/invoices",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_expired_invoices(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<u64>, ApplicationError> {
    let n_deleted = app_state
        .services
        .invoice
        .delete_many(InvoiceFilter {
            wallet_id: Some(user.sub),
            status: Some(InvoiceStatus::Expired),
            ..Default::default()
        })
        .await?;
    Ok(n_deleted.into())
}

/// Delete failed payments
///
/// Deletes all the payments with `Failed` status. Returns the number of deleted payments
#[utoipa::path(
    delete,
    path = "/payments",
    tag = "Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_failed_payments(
    State(app_state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<u64>, ApplicationError> {
    let n_deleted = app_state
        .services
        .payment
        .delete_many(PaymentFilter {
            wallet_id: Some(user.sub),
            status: Some(PaymentStatus::Failed),
            ..Default::default()
        })
        .await?;
    Ok(n_deleted.into())
}
