use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use axum_extra::extract::Query;
use utoipa::OpenApi;
use uuid::Uuid;

use swissknife_types::ErrorResponse;

use super::{
    CreateWebhookSubscriptionRequest, CreatedWebhookSubscription, RotateWebhookSecretResponse,
    UpdateWebhookSubscriptionRequest, WebhookDelivery, WebhookDeliveryDetails, WebhookDeliveryFilter,
    WebhookSubscription, WebhookSubscriptionFilter,
};
use crate::{
    application::{
        composition::AppServices,
        docs::{
            BAD_REQUEST_EXAMPLE, CONFLICT_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE,
            UNAUTHORIZED_EXAMPLE, UNPROCESSABLE_EXAMPLE,
        },
        errors::{ApplicationError, DataError},
    },
    domains::account::{Permission, User},
    infra::axum::{Json, Path},
};

#[derive(OpenApi)]
#[openapi(
    paths(create_webhook_subscription, get_webhook_subscription, list_webhook_subscriptions,
        update_webhook_subscription, delete_webhook_subscription, rotate_webhook_subscription_secret,
        list_webhook_subscription_deliveries, get_webhook_subscription_delivery, send_webhook_subscription_test, retry_webhook_subscription_delivery),
    components(schemas(CreateWebhookSubscriptionRequest, CreatedWebhookSubscription,
        UpdateWebhookSubscriptionRequest, RotateWebhookSecretResponse, WebhookSubscription, WebhookDelivery)),
    tags((name = "Webhooks", description = "Webhook management across accounts. Requires `read:webhook` or `write:webhook` permissions."))
)]
pub struct WebhookHandler;
pub const CONTEXT_PATH: &str = "/v1/webhooks";

pub fn webhook_router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", post(create_webhook_subscription).get(list_webhook_subscriptions))
        .route(
            "/{id}",
            get(get_webhook_subscription)
                .put(update_webhook_subscription)
                .delete(delete_webhook_subscription),
        )
        .route("/{id}/rotate-secret", post(rotate_webhook_subscription_secret))
        .route("/{id}/deliveries", get(list_webhook_subscription_deliveries))
        .route("/{id}/deliveries/{delivery_id}", get(get_webhook_subscription_delivery))
        .route("/{id}/test", post(send_webhook_subscription_test))
        .route(
            "/{id}/deliveries/{delivery_id}/retry",
            post(retry_webhook_subscription_delivery),
        )
}

/// Create a webhook for a wallet owned by any account.
#[utoipa::path(
    post,
    path = "",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    request_body = CreateWebhookSubscriptionRequest,
    responses(
        (status = 201, description = "Created; save the signing secret because it is returned only once", body = CreatedWebhookSubscription),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 409, description = "A subscription already exists for this wallet and URL", body = ErrorResponse, example = json!(CONFLICT_EXAMPLE)),
        (status = 422, description = "Invalid URL or empty event filter", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn create_webhook_subscription(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(request): Json<CreateWebhookSubscriptionRequest>,
) -> Result<(StatusCode, Json<CreatedWebhookSubscription>), ApplicationError> {
    user.check_permission(Permission::WriteWebhook)?;

    let wallet_id = request
        .wallet_id
        .ok_or_else(|| DataError::Malformed("wallet_id is required.".to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(services.webhook.create(wallet_id, request).await?),
    ))
}

/// Get a webhook subscription.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = WebhookSubscription),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_webhook_subscription(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<WebhookSubscription>, ApplicationError> {
    user.check_permission(Permission::ReadWebhook)?;

    Ok(Json(services.webhook.get(id).await?))
}

/// List webhook subscriptions across accounts.
#[utoipa::path(
    get,
    path = "",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    params(WebhookSubscriptionFilter),
    responses(
        (status = 200, description = "Subscriptions", body = Vec<WebhookSubscription>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_webhook_subscriptions(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(filter): Query<WebhookSubscriptionFilter>,
) -> Result<Json<Vec<WebhookSubscription>>, ApplicationError> {
    user.check_permission(Permission::ReadWebhook)?;

    Ok(Json(services.webhook.list(filter).await?))
}

/// Update a webhook endpoint, event filter, or enabled state.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    request_body = UpdateWebhookSubscriptionRequest,
    responses(
        (status = 200, description = "Updated", body = WebhookSubscription),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 409, description = "A subscription already exists for this wallet and URL", body = ErrorResponse, example = json!(CONFLICT_EXAMPLE)),
        (status = 422, description = "Invalid URL or empty event filter", body = ErrorResponse, example = json!(UNPROCESSABLE_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn update_webhook_subscription(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateWebhookSubscriptionRequest>,
) -> Result<Json<WebhookSubscription>, ApplicationError> {
    user.check_permission(Permission::WriteWebhook)?;

    Ok(Json(services.webhook.update(id, request).await?))
}

/// Delete a webhook subscription.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    responses(
        (status = 204, description = "Deleted"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_webhook_subscription(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApplicationError> {
    user.check_permission(Permission::WriteWebhook)?;

    services.webhook.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Rotate a webhook signing secret.
#[utoipa::path(
    post,
    path = "/{id}/rotate-secret",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Rotated; save the new secret because it is returned only once", body = RotateWebhookSecretResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn rotate_webhook_subscription_secret(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<RotateWebhookSecretResponse>, ApplicationError> {
    user.check_permission(Permission::WriteWebhook)?;

    Ok(Json(services.webhook.rotate_secret(id).await?))
}

/// List webhook delivery history.
#[utoipa::path(
    get,
    path = "/{id}/deliveries",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    params(WebhookDeliveryFilter),
    responses(
        (status = 200, description = "Paginated delivery history (up to 100 per page)", body = Vec<WebhookDelivery>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_webhook_subscription_deliveries(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
    Query(filter): Query<WebhookDeliveryFilter>,
) -> Result<Json<Vec<WebhookDelivery>>, ApplicationError> {
    user.check_permission(Permission::ReadWebhook)?;

    Ok(Json(services.webhook.list_deliveries(id, filter).await?))
}

/// Inspect a retained webhook delivery and its exact payload.
#[utoipa::path(
    get,
    path = "/{id}/deliveries/{delivery_id}",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = WebhookDeliveryDetails),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_webhook_subscription_delivery(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((id, delivery_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<WebhookDeliveryDetails>, ApplicationError> {
    user.check_permission(Permission::ReadWebhook)?;
    Ok(Json(services.webhook.get_delivery(id, delivery_id).await?))
}

/// Queue a subscription-local test event through the normal delivery worker.
#[utoipa::path(
    post,
    path = "/{id}/test",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    responses(
        (status = 202, description = "Queued for delivery", body = WebhookDelivery),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 409, description = "Subscription disabled, delivery pending, limit reached, or attempt in flight", body = ErrorResponse, example = json!(CONFLICT_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn send_webhook_subscription_test(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<WebhookDelivery>), ApplicationError> {
    user.check_permission(Permission::WriteWebhook)?;
    Ok((StatusCode::ACCEPTED, Json(services.webhook.send_test(id).await?)))
}

/// Retry a terminal delivery using its existing ID and remaining attempt budget.
#[utoipa::path(
    post,
    path = "/{id}/deliveries/{delivery_id}/retry",
    tag = "Webhooks",
    context_path = CONTEXT_PATH,
    responses(
        (status = 202, description = "Queued for delivery", body = WebhookDelivery),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 409, description = "Subscription disabled, delivery pending, limit reached, or attempt in flight", body = ErrorResponse, example = json!(CONFLICT_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn retry_webhook_subscription_delivery(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((id, delivery_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<WebhookDelivery>), ApplicationError> {
    user.check_permission(Permission::WriteWebhook)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(services.webhook.retry_delivery(id, delivery_id).await?),
    ))
}
