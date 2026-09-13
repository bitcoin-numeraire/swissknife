use std::{collections::VecDeque, convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::State,
    http::{header::CACHE_CONTROL, HeaderMap, HeaderName, HeaderValue},
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
    routing::{delete, get, post, put},
    Router,
};
use futures_util::stream::{self, Stream};
use tokio::time::{sleep, Instant};
use tracing::warn;
use utoipa::OpenApi;
use uuid::Uuid;

use swissknife_types::{
    Account, AccountPreferences, ClientEvent, ClientEventStreamQuery, ClientEventType, CreateApiKeyRequest,
    CreateWalletRequest, ErrorResponse, NewBtcAddressRequest, NewInvoiceRequest, PaymentFeeEstimate,
    RegisterLnAddressRequest, SendPaymentRequest, UpdateAccountPreferencesRequest, UpdateAccountRequest,
    UpdateLnAddressRequest,
};

use crate::{
    application::{
        composition::AppServices,
        docs::{
            BAD_REQUEST_EXAMPLE, CONFLICT_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE,
            UNPROCESSABLE_EXAMPLE,
        },
        errors::{ApplicationError, DataError},
    },
    domains::{
        account::{ApiKey, ApiKeyFilter, User},
        bitcoin::{BtcAddress, BtcAddressFilter},
        invoice::{Invoice, InvoiceFilter, InvoiceStatus},
        ln_address::{LnAddress, LnAddressFilter},
        payment::{Payment, PaymentFilter, PaymentStatus},
    },
    infra::axum::{Json, Path, Query},
};

use super::{Balance, Contact, Wallet, WalletFilter};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_account,
        stream_account_events,
        update_current_account,
        get_account_preferences,
        update_account_preferences,
        get_account_address,
        register_account_address,
        update_account_address,
        delete_account_address,
        create_account_api_key,
        list_account_api_keys,
        get_account_api_key,
        revoke_account_api_key,
        revoke_account_api_keys,
        list_account_wallets,
        create_account_wallet,
        get_account_wallet,
        get_wallet_balance,
        new_wallet_btc_address,
        list_wallet_btc_addresses,
        new_wallet_invoice,
        list_wallet_invoices,
        get_wallet_invoice,
        delete_expired_invoices,
        wallet_pay,
        estimate_wallet_payment_fee,
        list_wallet_payments,
        get_wallet_payment,
        delete_failed_payments,
        list_contacts,
    ),
    components(schemas(
        Account,
        AccountPreferences,
        ClientEvent,
        ClientEventType,
        ClientEventStreamQuery,
        UpdateAccountRequest,
        UpdateAccountPreferencesRequest,
        CreateWalletRequest,
        Wallet,
        Balance,
        Contact,
        LnAddress,
        BtcAddress,
        SendPaymentRequest,
        PaymentFeeEstimate,
        NewInvoiceRequest,
        NewBtcAddressRequest,
        CreateApiKeyRequest,
        ApiKey
    )),
    tags(
        (name = "Me", description = "Authenticated account endpoints. Wallet operations require an explicit account-owned wallet selector.")
    ),
)]
pub struct AccountWalletHandler;
pub const CONTEXT_PATH: &str = "/v1/me";

const LAST_EVENT_ID: &str = "last-event-id";
const EVENT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const AUTH_RECHECK_INTERVAL: Duration = Duration::from_secs(15);
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

pub fn account_router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", get(get_account))
        .route("/events", get(stream_account_events))
        .route("/", put(update_current_account))
        .route("/preferences", get(get_account_preferences))
        .route("/preferences", put(update_account_preferences))
        .route("/lightning-address", get(get_account_address))
        .route("/lightning-address", post(register_account_address))
        .route("/lightning-address", put(update_account_address))
        .route("/lightning-address", delete(delete_account_address))
        .route("/api-keys", post(create_account_api_key))
        .route("/api-keys", get(list_account_api_keys))
        .route("/api-keys/{id}", get(get_account_api_key))
        .route("/api-keys/{id}", delete(revoke_account_api_key))
        .route("/api-keys", delete(revoke_account_api_keys))
        .route("/wallets", get(list_account_wallets))
        .route("/wallets", post(create_account_wallet))
        .route("/wallets/{wallet_id}", get(get_account_wallet))
        .route("/wallets/{wallet_id}/balance", get(get_wallet_balance))
        .route("/wallets/{wallet_id}/bitcoin/addresses", get(list_wallet_btc_addresses))
        .route("/wallets/{wallet_id}/bitcoin/addresses", post(new_wallet_btc_address))
        .route("/wallets/{wallet_id}/invoices", post(new_wallet_invoice))
        .route("/wallets/{wallet_id}/invoices", get(list_wallet_invoices))
        .route("/wallets/{wallet_id}/invoices/{id}", get(get_wallet_invoice))
        .route("/wallets/{wallet_id}/invoices", delete(delete_expired_invoices))
        .route("/wallets/{wallet_id}/payments", post(wallet_pay))
        .route(
            "/wallets/{wallet_id}/payments/fee-estimate",
            post(estimate_wallet_payment_fee),
        )
        .route("/wallets/{wallet_id}/payments", get(list_wallet_payments))
        .route("/wallets/{wallet_id}/payments/{id}", get(get_wallet_payment))
        .route("/wallets/{wallet_id}/payments", delete(delete_failed_payments))
        .route("/wallets/{wallet_id}/contacts", get(list_contacts))
}

/// Stream durable events for every wallet owned by the authenticated account.
///
/// A fresh connection starts after the latest committed account event. Send
/// `Last-Event-ID` on reconnect (or `after` for deliberate replay) to receive
/// missed events. If that cursor is older than the retained replay window,
/// refresh REST state and reconnect without a cursor.
#[utoipa::path(
    get,
    path = "/events",
    tag = "Me",
    context_path = CONTEXT_PATH,
    params(ClientEventStreamQuery),
    responses(
        (status = 200, description = "Server-sent event stream", body = ClientEvent, content_type = "text/event-stream"),
        (status = 400, description = "Invalid replay cursor", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 409, description = "Replay cursor has expired", body = ErrorResponse, example = json!(CONFLICT_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn stream_account_events(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query): Query<ClientEventStreamQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApplicationError> {
    let cursor = match replay_cursor(&headers, query.after)? {
        Some(cursor) => {
            services.client_event.ensure_cursor_available(cursor).await?;
            cursor
        }
        None => services.client_event.latest_id(user.account_id).await?,
    };

    let stream = account_event_stream(services, user.account_id, cursor, headers);
    let sse = Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_ALIVE_INTERVAL).text("keep-alive"));

    Ok((
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-cache, no-transform")),
            (
                HeaderName::from_static("x-accel-buffering"),
                HeaderValue::from_static("no"),
            ),
        ],
        sse,
    ))
}

fn replay_cursor(headers: &HeaderMap, query_after: Option<i32>) -> Result<Option<i32>, ApplicationError> {
    let Some(raw) = headers.get(LAST_EVENT_ID) else {
        return query_after.map(validate_cursor).transpose();
    };

    let raw = raw
        .to_str()
        .map_err(|_| DataError::Malformed("Last-Event-ID must be a non-negative integer.".to_string()))?;
    let cursor = raw
        .parse::<i32>()
        .map_err(|_| DataError::Malformed("Last-Event-ID must be a non-negative integer.".to_string()))?;

    Ok(Some(validate_cursor(cursor)?))
}

fn validate_cursor(cursor: i32) -> Result<i32, ApplicationError> {
    if cursor < 0 {
        return Err(DataError::Malformed("Event cursor must be a non-negative integer.".to_string()).into());
    }

    Ok(cursor)
}

struct EventStreamState {
    services: Arc<AppServices>,
    account_id: Uuid,
    cursor: i32,
    pending: VecDeque<ClientEvent>,
    headers: HeaderMap,
    authenticated_at: Instant,
}

fn account_event_stream(
    services: Arc<AppServices>,
    account_id: Uuid,
    cursor: i32,
    headers: HeaderMap,
) -> impl Stream<Item = Result<Event, Infallible>> {
    stream::unfold(
        EventStreamState {
            services,
            account_id,
            cursor,
            pending: VecDeque::new(),
            headers,
            authenticated_at: Instant::now(),
        },
        |mut state| async move {
            loop {
                if state.authenticated_at.elapsed() >= AUTH_RECHECK_INTERVAL {
                    let Ok(user) = User::authenticate_headers(&state.headers, &state.services).await else {
                        return None;
                    };
                    if user.account_id != state.account_id {
                        return None;
                    }
                    state.authenticated_at = Instant::now();
                }
                if let Some(client_event) = state.pending.pop_front() {
                    state.cursor = client_event.id.parse().unwrap_or(state.cursor);
                    match serde_json::to_string(&client_event) {
                        Ok(data) => {
                            let event = Event::default()
                                .id(client_event.id)
                                .event(client_event.event_type.to_string())
                                .data(data);
                            return Some((Ok(event), state));
                        }
                        Err(error) => {
                            warn!(%error, "Failed to serialize a durable client event");
                            continue;
                        }
                    }
                }

                match state
                    .services
                    .client_event
                    .list_after(state.account_id, state.cursor)
                    .await
                {
                    Ok(events) if !events.is_empty() => state.pending.extend(events),
                    Ok(_) => sleep(EVENT_POLL_INTERVAL).await,
                    Err(ApplicationError::Data(DataError::Conflict(error))) => {
                        warn!(%error, account_id = %state.account_id, "Client event cursor expired while streaming");
                        return None;
                    }
                    Err(error) => {
                        warn!(%error, account_id = %state.account_id, "Failed to read the client event log");
                        sleep(EVENT_POLL_INTERVAL).await;
                    }
                }
            }
        },
    )
}

/// Get account.
#[utoipa::path(
    get,
    path = "",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Account),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_account(State(services): State<Arc<AppServices>>, user: User) -> Result<Json<Account>, ApplicationError> {
    let mut account = services.account.get(user.account_id).await?;
    account.permissions = Some(user.permissions);
    Ok(Json(account))
}

/// Update your account profile.
#[utoipa::path(
    put,
    path = "",
    tag = "Me",
    context_path = CONTEXT_PATH,
    request_body = UpdateAccountRequest,
    responses(
        (status = 200, description = "Updated", body = Account),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn update_current_account(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(UpdateAccountRequest { display_name }): Json<UpdateAccountRequest>,
) -> Result<Json<Account>, ApplicationError> {
    let mut account = services.account.update(user.account_id, display_name).await?;
    account.permissions = Some(user.permissions);
    Ok(Json(account))
}

/// Get account preferences.
#[utoipa::path(
    get,
    path = "/preferences",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = AccountPreferences),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_account_preferences(
    State(services): State<Arc<AppServices>>,
    user: User,
) -> Result<Json<AccountPreferences>, ApplicationError> {
    let account = services.account.get(user.account_id).await?;
    let preferences = account
        .preferences
        .ok_or_else(|| DataError::Inconsistency("Account preferences missing.".to_string()))?;
    Ok(Json(preferences))
}

/// Replace account preferences.
#[utoipa::path(
    put,
    path = "/preferences",
    tag = "Me",
    context_path = CONTEXT_PATH,
    request_body = UpdateAccountPreferencesRequest,
    responses(
        (status = 200, description = "Updated", body = AccountPreferences),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn update_account_preferences(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<UpdateAccountPreferencesRequest>,
) -> Result<Json<AccountPreferences>, ApplicationError> {
    Ok(Json(
        services
            .account
            .update_preferences(user.account_id, payload.dashboard_settings)
            .await?,
    ))
}

/// List account wallets.
#[utoipa::path(
    get,
    path = "/wallets",
    tag = "Me",
    context_path = CONTEXT_PATH,
    params(WalletFilter),
    responses(
        (status = 200, description = "Success", body = Vec<Wallet>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_account_wallets(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(mut filter): Query<WalletFilter>,
) -> Result<Json<Vec<Wallet>>, ApplicationError> {
    filter.account_id = Some(user.account_id);
    Ok(Json(services.wallet.list(filter).await?))
}

/// Create or enable an asset wallet.
#[utoipa::path(
    post,
    path = "/wallets",
    tag = "Me",
    context_path = CONTEXT_PATH,
    request_body = CreateWalletRequest,
    responses(
        (status = 200, description = "Wallet Created", body = Wallet),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn create_account_wallet(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<CreateWalletRequest>,
) -> Result<Json<Wallet>, ApplicationError> {
    Ok(Json(services.wallet.create(user.account_id, payload.asset_id).await?))
}

/// Get one account wallet.
#[utoipa::path(
    get,
    path = "/wallets/{wallet_id}",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Wallet),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_account_wallet(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
) -> Result<Json<Wallet>, ApplicationError> {
    Ok(Json(
        services.wallet.get_by_account_id(user.account_id, wallet_id).await?,
    ))
}

/// Generate a new Bitcoin address for a wallet.
#[utoipa::path(
    post,
    path = "/wallets/{wallet_id}/bitcoin/addresses",
    tag = "Me",
    context_path = CONTEXT_PATH,
    request_body = NewBtcAddressRequest,
    responses(
        (status = 200, description = "Bitcoin Address Created", body = BtcAddress),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn new_wallet_btc_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Json(payload): Json<NewBtcAddressRequest>,
) -> Result<Json<BtcAddress>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    let address = services
        .bitcoin
        .new_deposit_address(wallet_id, payload.address_type)
        .await?;
    Ok(Json(address))
}

/// List Bitcoin addresses for a wallet.
#[utoipa::path(
    get,
    path = "/wallets/{wallet_id}/bitcoin/addresses",
    tag = "Me",
    context_path = CONTEXT_PATH,
    params(BtcAddressFilter),
    responses(
        (status = 200, description = "Success", body = Vec<BtcAddress>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallet_btc_addresses(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Query(mut filter): Query<BtcAddressFilter>,
) -> Result<Json<Vec<BtcAddress>>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    filter.wallet_id = Some(wallet_id);
    Ok(Json(services.bitcoin.list_addresses(filter).await?))
}

/// Send a payment from a wallet.
#[utoipa::path(
    post,
    path = "/wallets/{wallet_id}/payments",
    tag = "Me",
    context_path = CONTEXT_PATH,
    request_body = SendPaymentRequest,
    responses(
        (status = 200, description = "Payment Sent", body = Payment),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn wallet_pay(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Json(payload): Json<SendPaymentRequest>,
) -> Result<Json<Payment>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    let payment = services
        .payment
        .pay(payload.input, payload.amount_msat, payload.comment, wallet_id)
        .await?;

    Ok(Json(payment))
}

/// Estimate a payment fee for an account-owned wallet.
#[utoipa::path(
    post,
    path = "/wallets/{wallet_id}/payments/fee-estimate",
    tag = "Me",
    context_path = CONTEXT_PATH,
    request_body = SendPaymentRequest,
    responses(
        (status = 200, description = "Fee estimated", body = PaymentFeeEstimate),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn estimate_wallet_payment_fee(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Json(payload): Json<SendPaymentRequest>,
) -> Result<Json<PaymentFeeEstimate>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;
    let estimate = services
        .payment
        .estimate_fee(payload.input, payload.amount_msat, payload.comment, wallet_id)
        .await?;

    Ok(Json(estimate))
}

/// Get wallet balance.
#[utoipa::path(
    get,
    path = "/wallets/{wallet_id}/balance",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Balance),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_balance(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
) -> Result<Json<Balance>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    Ok(Json(services.wallet.get_balance(wallet_id).await?))
}

/// Generate a new invoice for a wallet.
#[utoipa::path(
    post,
    path = "/wallets/{wallet_id}/invoices",
    tag = "Me",
    context_path = CONTEXT_PATH,
    request_body = NewInvoiceRequest,
    responses(
        (status = 200, description = "Invoice Created", body = Invoice),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn new_wallet_invoice(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Json(payload): Json<NewInvoiceRequest>,
) -> Result<Json<Invoice>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    let invoice = services
        .invoice
        .invoice(wallet_id, payload.amount_msat, payload.description, payload.expiry)
        .await?;

    Ok(Json(invoice))
}

/// Get account Lightning Address.
#[utoipa::path(
    get,
    path = "/lightning-address",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Option<LnAddress>),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_account_address(
    State(services): State<Arc<AppServices>>,
    user: User,
) -> Result<Json<Option<LnAddress>>, ApplicationError> {
    let ln_addresses = services
        .ln_address
        .list(LnAddressFilter {
            account_id: Some(user.account_id),
            ..Default::default()
        })
        .await?;

    Ok(Json(ln_addresses.into_iter().next()))
}

/// Register account Lightning Address.
#[utoipa::path(
    post,
    path = "/lightning-address",
    tag = "Me",
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
async fn register_account_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<RegisterLnAddressRequest>,
) -> Result<Json<LnAddress>, ApplicationError> {
    let ln_address = services
        .ln_address
        .register(
            user.account_id,
            payload.username,
            payload.allows_nostr,
            payload.nostr_pubkey,
        )
        .await?;
    Ok(Json(ln_address))
}

/// Update account Lightning Address.
#[utoipa::path(
    put,
    path = "/lightning-address",
    tag = "Me",
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
async fn update_account_address(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<UpdateLnAddressRequest>,
) -> Result<Json<LnAddress>, ApplicationError> {
    let ln_addresses = services
        .ln_address
        .list(LnAddressFilter {
            account_id: Some(user.account_id),
            ..Default::default()
        })
        .await?;

    let ln_address = ln_addresses
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("LN Address not found.".to_string()))?;

    Ok(Json(services.ln_address.update(ln_address.id, payload).await?))
}

/// Delete account Lightning Address.
#[utoipa::path(
    delete,
    path = "/lightning-address",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Deleted"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_account_address(State(services): State<Arc<AppServices>>, user: User) -> Result<(), ApplicationError> {
    let n_deleted = services
        .ln_address
        .delete_many(LnAddressFilter {
            account_id: Some(user.account_id),
            ..Default::default()
        })
        .await?;

    if n_deleted == 0 {
        return Err(DataError::NotFound("Lightning address not found.".to_string()).into());
    }

    Ok(())
}

/// List payments for a wallet.
#[utoipa::path(
    get,
    path = "/wallets/{wallet_id}/payments",
    tag = "Me",
    context_path = CONTEXT_PATH,
    params(PaymentFilter),
    responses(
        (status = 200, description = "Success", body = Vec<Payment>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallet_payments(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Query(mut query_params): Query<PaymentFilter>,
) -> Result<Json<Vec<Payment>>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    query_params.wallet_id = Some(wallet_id);
    Ok(Json(services.payment.list(query_params).await?))
}

/// Get a wallet payment.
#[utoipa::path(
    get,
    path = "/wallets/{wallet_id}/payments/{id}",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Payment),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_payment(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((wallet_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Payment>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    let payments = services
        .payment
        .list(PaymentFilter {
            wallet_id: Some(wallet_id),
            ids: Some(vec![id]),
            ..Default::default()
        })
        .await?;

    let payment = payments
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("Payment not found.".to_string()))?;

    Ok(Json(payment))
}

/// List invoices for a wallet.
#[utoipa::path(
    get,
    path = "/wallets/{wallet_id}/invoices",
    tag = "Me",
    context_path = CONTEXT_PATH,
    params(InvoiceFilter),
    responses(
        (status = 200, description = "Success", body = Vec<Invoice>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_wallet_invoices(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Query(mut query_params): Query<InvoiceFilter>,
) -> Result<Json<Vec<Invoice>>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    query_params.wallet_id = Some(wallet_id);
    Ok(Json(services.invoice.list(query_params).await?))
}

/// Get a wallet invoice.
#[utoipa::path(
    get,
    path = "/wallets/{wallet_id}/invoices/{id}",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Invoice),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_wallet_invoice(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((wallet_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Invoice>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    let invoices = services
        .invoice
        .list(InvoiceFilter {
            wallet_id: Some(wallet_id),
            ids: Some(vec![id]),
            ..Default::default()
        })
        .await?;

    let invoice = invoices
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("Invoice not found.".to_string()))?;

    Ok(Json(invoice))
}

/// List contacts for a wallet.
#[utoipa::path(
    get,
    path = "/wallets/{wallet_id}/contacts",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = Vec<Contact>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_contacts(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
) -> Result<Json<Vec<Contact>>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    Ok(Json(services.wallet.list_contacts(wallet_id).await?))
}

/// Delete expired invoices for a wallet.
#[utoipa::path(
    delete,
    path = "/wallets/{wallet_id}/invoices",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_expired_invoices(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
) -> Result<Json<u64>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    let n_deleted = services
        .invoice
        .delete_many(InvoiceFilter {
            wallet_id: Some(wallet_id),
            status: Some(InvoiceStatus::Expired),
            ..Default::default()
        })
        .await?;
    Ok(Json(n_deleted))
}

/// Delete failed payments for a wallet.
#[utoipa::path(
    delete,
    path = "/wallets/{wallet_id}/payments",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_failed_payments(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
) -> Result<Json<u64>, ApplicationError> {
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    let n_deleted = services
        .payment
        .delete_many(PaymentFilter {
            wallet_id: Some(wallet_id),
            status: Some(PaymentStatus::Failed),
            ..Default::default()
        })
        .await?;
    Ok(Json(n_deleted))
}

/// Generate a new API Key
///
/// Returns the generated API Key for the account. Callers can create API keys with
/// permissions as a subset of their current permissions.
#[utoipa::path(
    post,
    path = "/api-keys",
    tag = "Me",
    context_path = CONTEXT_PATH,
    request_body = CreateApiKeyRequest,
    responses(
        (status = 200, description = "API Key Created", body = ApiKey),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn create_account_api_key(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(mut payload): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKey>, ApplicationError> {
    payload.account_id = Some(user.account_id);
    let api_key = services.api_key.generate(user, payload).await?;
    Ok(Json(api_key))
}

/// Get an account API key.
#[utoipa::path(
    get,
    path = "/api-keys/{id}",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = ApiKey),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_account_api_key(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiKey>, ApplicationError> {
    let api_keys = services
        .api_key
        .list(ApiKeyFilter {
            account_id: Some(user.account_id),
            ids: Some(vec![id]),
            ..Default::default()
        })
        .await?;

    let api_key = api_keys
        .first()
        .cloned()
        .ok_or_else(|| DataError::NotFound("API Key not found.".to_string()))?;

    Ok(Json(api_key))
}

/// List account API keys.
#[utoipa::path(
    get,
    path = "/api-keys",
    tag = "Me",
    context_path = CONTEXT_PATH,
    params(ApiKeyFilter),
    responses(
        (status = 200, description = "Success", body = Vec<ApiKey>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_account_api_keys(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(mut filter): Query<ApiKeyFilter>,
) -> Result<Json<Vec<ApiKey>>, ApplicationError> {
    filter.account_id = Some(user.account_id);
    let api_keys = services.api_key.list(filter).await?;

    Ok(Json(api_keys))
}

/// Revoke an account API key.
#[utoipa::path(
    delete,
    path = "/api-keys/{id}",
    tag = "Me",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Revoked"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn revoke_account_api_key(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    let n_revoked = services
        .api_key
        .revoke_many(ApiKeyFilter {
            account_id: Some(user.account_id),
            ids: Some(vec![id]),
            ..Default::default()
        })
        .await?;

    if n_revoked == 0 {
        return Err(DataError::NotFound("API Key not found.".to_string()).into());
    }

    Ok(())
}

/// Revoke account API keys.
#[utoipa::path(
    delete,
    path = "/api-keys",
    tag = "Me",
    context_path = CONTEXT_PATH,
    params(ApiKeyFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn revoke_account_api_keys(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(mut filter): Query<ApiKeyFilter>,
) -> Result<Json<u64>, ApplicationError> {
    filter.account_id = Some(user.account_id);
    let n_revoked = services.api_key.revoke_many(filter).await?;
    Ok(n_revoked.into())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        application::composition::MockAppServicesBuilder,
        domains::{
            bitcoin::{BtcAddress, BtcAddressType},
            payment::Payment,
        },
    };

    use super::*;

    fn user() -> User {
        User {
            account_id: Uuid::new_v4(),
            permissions: vec![],
        }
    }

    fn btc_address(wallet_id: Uuid) -> BtcAddress {
        BtcAddress {
            id: Uuid::new_v4(),
            wallet_id,
            address: "bcrt1qexample".to_string(),
            used: false,
            address_type: BtcAddressType::P2wpkh,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    fn fee_estimate() -> PaymentFeeEstimate {
        PaymentFeeEstimate {
            ledger: crate::application::composition::Ledger::Lightning,
            amount_msat: 1_000,
            estimated_fee_msat: Some(10),
            maximum_fee_msat: 100,
            estimated_total_msat: Some(1_010),
            maximum_total_msat: 1_100,
        }
    }

    mod update_account {
        use super::*;

        #[tokio::test]
        async fn updates_only_the_authenticated_account() {
            let caller = user();
            let account_id = caller.account_id;
            let mut builder = MockAppServicesBuilder::new();
            builder
                .account
                .expect_update()
                .withf(move |id, display_name| *id == account_id && display_name.as_deref() == Some("Alice"))
                .times(1)
                .returning(move |id, display_name| {
                    Ok(Account {
                        id,
                        display_name,
                        ..Default::default()
                    })
                });

            let result = super::update_current_account(
                State(Arc::new(builder.build())),
                caller,
                Json(UpdateAccountRequest {
                    display_name: Some("Alice".to_string()),
                }),
            )
            .await
            .unwrap();

            assert_eq!(result.0.display_name.as_deref(), Some("Alice"));
        }
    }

    mod get_account {
        use super::*;

        #[tokio::test]
        async fn delegates_to_account_service() {
            let caller = user();
            let account_id = caller.account_id;

            let mut builder = MockAppServicesBuilder::new();
            builder.account.expect_get().times(1).returning(move |id| {
                assert_eq!(id, account_id);
                Ok(Account {
                    id,
                    display_name: None,
                    identity: None,
                    permissions: None,
                    preferences: None,
                    wallets: Vec::new(),
                    created_at: Utc::now(),
                    updated_at: None,
                })
            });

            let result = super::get_account(State(Arc::new(builder.build())), caller).await;

            assert!(result.is_ok());
        }
    }

    mod wallet_pay {
        use super::*;

        #[tokio::test]
        async fn uses_the_path_wallet_after_ownership_check() {
            let caller = user();
            let account_id = caller.account_id;
            let wallet_id = Uuid::new_v4();

            let mut builder = MockAppServicesBuilder::new();
            builder
                .wallet
                .expect_verify_ownership()
                .withf(move |account, id| *account == account_id && *id == wallet_id)
                .times(1)
                .returning(|_, _| Ok(()));
            builder
                .payment
                .expect_pay()
                .withf(move |_, _, _, id| *id == wallet_id)
                .times(1)
                .returning(|_, _, _, _| Ok(Payment::default()));

            let payload = SendPaymentRequest {
                wallet_id: None,
                input: "bob@numeraire.tech".to_string(),
                amount_msat: Some(1_000),
                comment: None,
            };

            let result =
                super::wallet_pay(State(Arc::new(builder.build())), caller, Path(wallet_id), Json(payload)).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn rejects_wallets_outside_the_account_scope() {
            let caller = user();
            let account_id = caller.account_id;
            let wallet_id = Uuid::new_v4();

            let mut builder = MockAppServicesBuilder::new();
            builder
                .wallet
                .expect_verify_ownership()
                .withf(move |account, id| *account == account_id && *id == wallet_id)
                .times(1)
                .returning(|_, _| Err(DataError::NotFound("Wallet not found.".to_string()).into()));

            let payload = SendPaymentRequest {
                wallet_id: None,
                input: "bob@numeraire.tech".to_string(),
                amount_msat: Some(1_000),
                comment: None,
            };

            let result =
                super::wallet_pay(State(Arc::new(builder.build())), caller, Path(wallet_id), Json(payload)).await;

            assert!(matches!(result, Err(ApplicationError::Data(_))));
        }
    }

    mod estimate_wallet_payment_fee {
        use super::*;

        #[tokio::test]
        async fn uses_the_path_wallet_after_ownership_check() {
            let caller = user();
            let account_id = caller.account_id;
            let wallet_id = Uuid::new_v4();

            let mut builder = MockAppServicesBuilder::new();
            builder
                .wallet
                .expect_verify_ownership()
                .withf(move |account, id| *account == account_id && *id == wallet_id)
                .times(1)
                .returning(|_, _| Ok(()));
            builder
                .payment
                .expect_estimate_fee()
                .withf(move |_, _, _, id| *id == wallet_id)
                .times(1)
                .returning(|_, _, _, _| Ok(fee_estimate()));

            let payload = SendPaymentRequest {
                wallet_id: Some(Uuid::new_v4()),
                input: "bob@numeraire.tech".to_string(),
                amount_msat: Some(1_000),
                comment: None,
            };

            let result = super::estimate_wallet_payment_fee(
                State(Arc::new(builder.build())),
                caller,
                Path(wallet_id),
                Json(payload),
            )
            .await
            .unwrap();

            assert_eq!(result.0.maximum_fee_msat, 100);
        }

        #[tokio::test]
        async fn rejects_wallets_outside_the_account_scope() {
            let caller = user();
            let wallet_id = Uuid::new_v4();
            let mut builder = MockAppServicesBuilder::new();
            builder
                .wallet
                .expect_verify_ownership()
                .times(1)
                .returning(|_, _| Err(DataError::NotFound("Wallet not found.".to_string()).into()));

            let payload = SendPaymentRequest {
                wallet_id: None,
                input: "bob@numeraire.tech".to_string(),
                amount_msat: Some(1_000),
                comment: None,
            };

            let result = super::estimate_wallet_payment_fee(
                State(Arc::new(builder.build())),
                caller,
                Path(wallet_id),
                Json(payload),
            )
            .await;

            assert!(matches!(result, Err(ApplicationError::Data(_))));
        }
    }

    mod new_wallet_btc_address {
        use super::*;

        #[tokio::test]
        async fn scopes_address_derivation_to_the_path_wallet() {
            let caller = user();
            let account_id = caller.account_id;
            let wallet_id = Uuid::new_v4();

            let mut builder = MockAppServicesBuilder::new();
            builder
                .wallet
                .expect_verify_ownership()
                .withf(move |account, id| *account == account_id && *id == wallet_id)
                .times(1)
                .returning(|_, _| Ok(()));
            builder
                .bitcoin
                .expect_new_deposit_address()
                .withf(move |id, _| *id == wallet_id)
                .times(1)
                .returning(|id, _| Ok(btc_address(id)));

            let payload = NewBtcAddressRequest {
                wallet_id: None,
                address_type: None,
            };

            let result =
                super::new_wallet_btc_address(State(Arc::new(builder.build())), caller, Path(wallet_id), Json(payload))
                    .await;

            assert!(result.is_ok());
        }
    }

    mod replay_cursor {
        use super::*;

        #[test]
        fn last_event_id_takes_precedence_over_query_cursor() {
            let mut headers = HeaderMap::new();
            headers.insert(LAST_EVENT_ID, HeaderValue::from_static("42"));

            assert_eq!(replay_cursor(&headers, Some(7)).unwrap(), Some(42));
        }

        #[test]
        fn rejects_invalid_last_event_id() {
            let mut headers = HeaderMap::new();
            headers.insert(LAST_EVENT_ID, HeaderValue::from_static("not-an-id"));

            assert!(matches!(
                replay_cursor(&headers, None),
                Err(ApplicationError::Data(DataError::Malformed(_)))
            ));
        }

        #[test]
        fn rejects_a_negative_query_cursor() {
            assert!(matches!(
                replay_cursor(&HeaderMap::new(), Some(-1)),
                Err(ApplicationError::Data(DataError::Malformed(_)))
            ));
        }
    }

    mod stream_account_events {
        use super::*;

        mod without_a_replay_cursor {
            use super::*;

            #[tokio::test]
            async fn fresh_stream_starts_at_the_accounts_latest_event() {
                let caller = user();
                let account_id = caller.account_id;
                let mut services = MockAppServicesBuilder::new();
                services
                    .client_event
                    .expect_latest_id()
                    .withf(move |account| *account == account_id)
                    .times(1)
                    .returning(|_| Ok(42));

                let result = stream_account_events(
                    State(Arc::new(services.build())),
                    caller,
                    Query(ClientEventStreamQuery { after: None }),
                    HeaderMap::new(),
                )
                .await;

                assert!(result.is_ok());
            }
        }

        mod with_a_replay_cursor {
            use super::*;

            #[tokio::test]
            async fn explicit_replay_validates_the_cursor_before_opening() {
                let caller = user();
                let mut services = MockAppServicesBuilder::new();
                services
                    .client_event
                    .expect_ensure_cursor_available()
                    .withf(|cursor| *cursor == 42)
                    .times(1)
                    .returning(|_| Ok(()));

                let result = stream_account_events(
                    State(Arc::new(services.build())),
                    caller,
                    Query(ClientEventStreamQuery { after: Some(42) }),
                    HeaderMap::new(),
                )
                .await;

                assert!(result.is_ok());
            }
        }
    }
}
