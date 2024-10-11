use std::sync::Arc;

use axum::{
    extract::State,
    routing::{delete, get, post},
    Router,
};
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    application::{
        docs::{
            BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE,
            UNAUTHORIZED_EXAMPLE, UNPROCESSABLE_EXAMPLE,
        },
        dtos::{InvoiceResponse, LnInvoiceResponse, NewInvoiceRequest},
        errors::ApplicationError,
    },
    domains::user::{Permission, User},
    infra::{
        app::AppState,
        axum::{Json, Path, Query},
    },
};

use super::{InvoiceFilter, InvoiceOrderBy, InvoiceStatus};

#[derive(OpenApi)]
#[openapi(
    paths(generate_invoice, list_invoices, get_invoice, delete_invoice, delete_invoices),
    components(schemas(InvoiceResponse, NewInvoiceRequest, InvoiceStatus, LnInvoiceResponse, InvoiceOrderBy)),
    tags(
        (name = "Invoices", description = "Invoice management endpoints. Require `read:transaction` or `write:transaction` permissions.")
    ),
)]
pub struct InvoiceHandler;
pub const CONTEXT_PATH: &str = "/v1/invoices";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(generate_invoice))
        .route("/", get(list_invoices))
        .route("/:id", get(get_invoice))
        .route("/:id", delete(delete_invoice))
        .route("/", delete(delete_invoices))
}

/// Generate a new invoice
///
/// Returns the generated invoice for the given user
#[utoipa::path(
    post,
    path = "",
    tag = "Invoices",
    context_path = CONTEXT_PATH,
    request_body = NewInvoiceRequest,
    responses(
        (status = 200, description = "Invoice Created", body = InvoiceResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 422, description = "Unprocessable Entity", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn generate_invoice(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Json(payload): Json<NewInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, ApplicationError> {
    user.check_permission(Permission::WriteLnTransaction)?;

    let invoice = app_state
        .services
        .invoice
        .invoice(
            payload.wallet_id.unwrap_or(user.wallet_id),
            payload.amount_msat,
            payload.description,
            payload.expiry,
        )
        .await?;
    Ok(Json(invoice.into()))
}

/// Find an invoice
///
/// Returns the invoice by its ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Invoices",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = InvoiceResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_invoice(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<InvoiceResponse>, ApplicationError> {
    user.check_permission(Permission::ReadLnTransaction)?;

    let invoice = app_state.services.invoice.get(id).await?;
    Ok(Json(invoice.into()))
}

/// List invoices
///
/// Returns all the invoices given a filter
#[utoipa::path(
    get,
    path = "",
    tag = "Invoices",
    context_path = CONTEXT_PATH,
    params(InvoiceFilter),
    responses(
        (status = 200, description = "Success", body = Vec<InvoiceResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_invoices(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(filter): Query<InvoiceFilter>,
) -> Result<Json<Vec<InvoiceResponse>>, ApplicationError> {
    user.check_permission(Permission::ReadLnTransaction)?;

    let invoices = app_state.services.invoice.list(filter).await?;
    let response: Vec<InvoiceResponse> = invoices.into_iter().map(Into::into).collect();

    Ok(response.into())
}

/// Delete an invoice
///
/// Deletes an invoice by ID. Returns an empty body. Deleting an invoice has an effect on the user balance
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Invoices",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Deleted"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_invoice(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteLnTransaction)?;

    app_state.services.invoice.delete(id).await?;
    Ok(())
}

/// Delete invoices
///
/// Deletes all the invoices given a filter. Returns the number of deleted invoices. Deleting an invoice can have an effect on the user balance
#[utoipa::path(
    delete,
    path = "",
    tag = "Invoices",
    context_path = CONTEXT_PATH,
    params(InvoiceFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_invoices(
    State(app_state): State<Arc<AppState>>,
    user: User,
    Query(query_params): Query<InvoiceFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteLnTransaction)?;

    let n_deleted = app_state.services.invoice.delete_many(query_params).await?;
    Ok(n_deleted.into())
}
