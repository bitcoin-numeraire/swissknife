use std::sync::Arc;

use axum::{
    extract::State,
    routing::{delete, get, post, put},
    Router,
};
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    application::{
        docs::{BAD_REQUEST_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE, UNPROCESSABLE_EXAMPLE},
        dtos::{
            ApiKeyResponse, BitcoinAddressResponse, CreateApiKeyRequest, ErrorResponse, InvoiceResponse,
            NewInvoiceRequest, PaymentResponse, RegisterLnAddressRequest, SendPaymentRequest, UpdateLnAddressRequest,
            WalletLnAddressResponse, WalletResponse, BitcoinAddressQueryParams,
        },
        errors::{ApplicationError, DataError},
    },
    domains::{
        invoice::{InvoiceFilter, InvoiceStatus},
        ln_address::{LnAddress, LnAddressFilter},
        payment::{PaymentFilter, PaymentStatus},
        user::{ApiKeyFilter, User},
    },
    infra::{
        app::AppState,
        axum::{Json, Path, Query},
    },
};

use super::{Balance, Contact};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_user_wallet, get_bitcoin_deposit_address,
        get_wallet_balance,
        get_wallet_address, register_wallet_address, update_wallet_address, delete_wallet_address,
        wallet_pay, list_wallet_payments, get_wallet_payment, delete_failed_payments, list_wallet_invoices,
        get_wallet_invoice, new_wallet_invoice, delete_expired_invoices,
        list_contacts,
        create_wallet_api_key, list_wallet_api_keys, get_wallet_api_key, revoke_wallet_api_key, revoke_wallet_api_keys
    ),
    components(schemas(WalletResponse, Balance, Contact, WalletLnAddressResponse, BitcoinAddressResponse)),
    tags(
        (name = "User Wallet", description = "User Wallet endpoints. Available to any authenticated user.")
    ),
)]
pub struct UserWalletHandler;
pub const CONTEXT_PATH: &str = "/v1/me";

pub fn user_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_user_wallet))
        .route("/bitcoin/address", get(get_bitcoin_deposit_address))
        .route("/balance", get(get_wallet_balance))
        .route("/lightning-address", get(get_wallet_address))
        .route("/lightning-address", post(register_wallet_address))
        .route("/lightning-address", put(update_wallet_address))
        .route("/lightning-address", delete(delete_wallet_address))
        .route("/payments", post(wallet_pay))
        .route("/payments", get(list_wallet_payments))
        .route("/payments/:id", get(get_wallet_payment))
        .route("/payments", delete(delete_failed_payments))
        .route("/invoices", post(new_wallet_invoice))
        .route("/invoices", get(list_wallet_invoices))
        .route("/invoices/:id", get(get_wallet_invoice))
        .route("/invoices", delete(delete_expired_invoices))
        .route("/contacts", get(list_contacts))
        .route("/api-keys", post(create_wallet_api_key))
        .route("/api-keys", get(list_wallet_api_keys))
        .route("/api-keys/:id", get(get_wallet_api_key))
        .route("/api-keys/:id", delete(revoke_wallet_api_key))
        .route("/api-keys", delete(revoke_wallet_api_keys))
}

/// Get wallet
///
/// Returns the user wallet.
#[utoipa::path(
    get,
    path = "",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = WalletResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_user_wallet(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<WalletResponse>, ApplicationError> {
    let wallet = app_state.services.wallet.get(user.wallet_id).await?;
    Ok(Json(wallet.into()))
}

/// Get current Bitcoin deposit address
///
/// Returns the active onchain deposit address for the authenticated wallet.
#[utoipa::path(
    get,
    path = "/bitcoin/address",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    params(BitcoinAddressQueryParams),
    responses(
        (status = 200, description = "Found", body = BitcoinAddressResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_bitcoin_deposit_address(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(query_params): Query<BitcoinAddressQueryParams>,
) -> Result<Json<BitcoinAddressResponse>, ApplicationError> {
    let address = app_state.services.bitcoin.get_deposit_address(
        user.wallet_id, 
        query_params.address_type.map(Into::into)
    ).await?;

    Ok(Json(address.into()))
}

/// Send payment
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
    user: User,
    Json(payload): Json<SendPaymentRequest>,
) -> Result<Json<PaymentResponse>, ApplicationError> {
    let payment = app_state
        .services
        .payment
        .pay(payload.input, payload.amount_msat, payload.comment, user.wallet_id)
        .await?;

    Ok(Json(payment.into()))
}

/// Get wallet balance
///
/// Returns the wallet balance.
#[utoipa::path(
    get,
    path = "/balance",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Balance),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_balance(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Balance>, ApplicationError> {
    let balance = app_state.services.wallet.get_balance(user.wallet_id).await?;
    Ok(balance.into())
}

/// Generate a new invoice
///
/// Returns the generated invoice
#[utoipa::path(
    post,
    path = "/invoices",
    tag = "User Wallet",
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
    user: User,
    Json(payload): Json<NewInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, ApplicationError> {
    let invoice = app_state
        .services
        .invoice
        .invoice(user.wallet_id, payload.amount_msat, payload.description, payload.expiry)
        .await?;

    Ok(Json(invoice.into()))
}

/// Get LN Address
///
/// Returns the registered address
#[utoipa::path(
    get,
    path = "/lightning-address",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = WalletLnAddressResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_address(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<WalletLnAddressResponse>, ApplicationError> {
    let ln_addresses = app_state
        .services
        .ln_address
        .list(LnAddressFilter {
            wallet_id: Some(user.wallet_id),
            ..Default::default()
        })
        .await?;

    let ln_address = ln_addresses.first().cloned();

    Ok(WalletLnAddressResponse { ln_address }.into())
}

/// Register LN Address
///
/// Registers an address. Returns the address details. LN Addresses are ready to receive funds through the LNURL protocol upon registration.
#[utoipa::path(
    post,
    path = "/lightning-address",
    tag = "User Wallet",
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
    user: User,
    Json(payload): Json<RegisterLnAddressRequest>,
) -> Result<Json<LnAddress>, ApplicationError> {
    let ln_address = app_state
        .services
        .ln_address
        .register(
            user.wallet_id,
            payload.username,
            payload.allows_nostr,
            payload.nostr_pubkey,
        )
        .await?;
    Ok(ln_address.into())
}

/// Update LN Address
///
/// Updates the address. Returns the address details.
#[utoipa::path(
    put,
    path = "/lightning-address",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    request_body = UpdateLnAddressRequest,
    responses(
        (status = 200, description = "LN Address Updated", body = LnAddress),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn update_wallet_address(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<UpdateLnAddressRequest>,
) -> Result<Json<LnAddress>, ApplicationError> {
    let ln_addresses = app_state
        .services
        .ln_address
        .list(LnAddressFilter {
            wallet_id: Some(user.wallet_id),
            ..Default::default()
        })
        .await?;

    let ln_address = ln_addresses
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("LN Address not found.".to_string()))?;

    let ln_address = app_state.services.ln_address.update(ln_address.id, payload).await?;

    Ok(ln_address.into())
}

/// Delete LN Address
///
/// Deletes an address. Returns an empty body. Once the address is deleted, it will no longer be able to receive funds and its username can be claimed by another user.
#[utoipa::path(
    delete,
    path = "/lightning-address",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Deleted"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_wallet_address(State(app_state): State<Arc<AppState>>, user: User) -> Result<(), ApplicationError> {
    let n_deleted = app_state
        .services
        .ln_address
        .delete_many(LnAddressFilter {
            wallet_id: Some(user.wallet_id),
            ..Default::default()
        })
        .await?;

    if n_deleted == 0 {
        return Err(DataError::NotFound("Lightning address not found.".to_string()).into());
    }

    Ok(())
}

/// List payments
///
/// Returns all the payments given a filter
#[utoipa::path(
    get,
    path = "/payments",
    tag = "User Wallet",
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
    user: User,
    Query(mut query_params): Query<PaymentFilter>,
) -> Result<Json<Vec<PaymentResponse>>, ApplicationError> {
    query_params.wallet_id = Some(user.wallet_id);
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
    tag = "User Wallet",
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
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<PaymentResponse>, ApplicationError> {
    let payments = app_state
        .services
        .payment
        .list(PaymentFilter {
            wallet_id: Some(user.wallet_id),
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
    tag = "User Wallet",
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
    user: User,
    Query(mut query_params): Query<InvoiceFilter>,
) -> Result<Json<Vec<InvoiceResponse>>, ApplicationError> {
    query_params.wallet_id = Some(user.wallet_id);
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
    tag = "User Wallet",
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
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<InvoiceResponse>, ApplicationError> {
    let invoices = app_state
        .services
        .invoice
        .list(InvoiceFilter {
            wallet_id: Some(user.wallet_id),
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
    tag = "User Wallet",
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
    user: User,
) -> Result<Json<Vec<Contact>>, ApplicationError> {
    let contacts = app_state.services.wallet.list_contacts(user.wallet_id).await?;
    Ok(contacts.into())
}

/// Delete expired invoices
///
/// Deletes all the invoices with status `Èxpired`. Returns the number of deleted invoices
#[utoipa::path(
    delete,
    path = "/invoices",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_expired_invoices(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<u64>, ApplicationError> {
    let n_deleted = app_state
        .services
        .invoice
        .delete_many(InvoiceFilter {
            wallet_id: Some(user.wallet_id),
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
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_failed_payments(
    State(app_state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<u64>, ApplicationError> {
    let n_deleted = app_state
        .services
        .payment
        .delete_many(PaymentFilter {
            wallet_id: Some(user.wallet_id),
            status: Some(PaymentStatus::Failed),
            ..Default::default()
        })
        .await?;
    Ok(n_deleted.into())
}

/// Generate a new API Key
///
/// Returns the generated API Key for the given user. Users can create API keys with permissions as a subset of his current permissions.
#[utoipa::path(
    post,
    path = "/api-keys",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    request_body = CreateApiKeyRequest,
    responses(
        (status = 200, description = "API Key Created", body = ApiKeyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn create_wallet_api_key(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(mut payload): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, ApplicationError> {
    payload.user_id = Some(user.id.clone());
    let api_key = app_state.services.api_key.generate(user, payload).await?;
    Ok(Json(api_key.into()))
}

/// Find an API Key
///
/// Returns the API Key by its ID.
#[utoipa::path(
    get,
    path = "/api-keys/{id}",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = ApiKeyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_api_key(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiKeyResponse>, ApplicationError> {
    let api_keys = app_state
        .services
        .api_key
        .list(ApiKeyFilter {
            user_id: Some(user.id),
            ids: Some(vec![id]),
            ..Default::default()
        })
        .await?;

    let api_key = api_keys
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("API Key not found.".to_string()))?;

    Ok(Json(api_key.into()))
}

/// List API Keys
///
/// Returns all the API Keys given a filter
#[utoipa::path(
    get,
    path = "/api-keys",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    params(ApiKeyFilter),
    responses(
        (status = 200, description = "Success", body = Vec<ApiKeyResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallet_api_keys(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(mut filter): Query<ApiKeyFilter>,
) -> Result<Json<Vec<ApiKeyResponse>>, ApplicationError> {
    filter.user_id = Some(user.id);
    let api_keys = app_state.services.api_key.list(filter).await?;
    let response: Vec<ApiKeyResponse> = api_keys.into_iter().map(Into::into).collect();

    Ok(response.into())
}

/// Revoke an API Key
///
/// Revokes an API Key by ID. Returns an empty body.
#[utoipa::path(
    delete,
    path = "/api-keys/{id}",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Revoked"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn revoke_wallet_api_key(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    let n_revoked = app_state
        .services
        .api_key
        .revoke_many(ApiKeyFilter {
            user_id: Some(user.id),
            ids: Some(vec![id]),
            ..Default::default()
        })
        .await?;

    if n_revoked == 0 {
        return Err(DataError::NotFound("API Key not found.".to_string()).into());
    }

    Ok(())
}

/// Revoke API Keys
///
/// Revokes all the API Keys given a filter. Returns the number of revoked keys.
#[utoipa::path(
    delete,
    path = "/api-keys",
    tag = "User Wallet",
    context_path = CONTEXT_PATH,
    params(ApiKeyFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn revoke_wallet_api_keys(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(mut filter): Query<ApiKeyFilter>,
) -> Result<Json<u64>, ApplicationError> {
    filter.user_id = Some(user.id);
    let n_revoked = app_state.services.api_key.revoke_many(filter).await?;
    Ok(n_revoked.into())
}
