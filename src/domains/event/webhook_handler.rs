use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post, put},
    Router,
};
use utoipa::OpenApi;
use uuid::Uuid;

use swissknife_types::{
    CreateWebhookSubscriptionRequest, CreatedWebhookSubscription, ErrorResponse, RotateWebhookSecretResponse,
    UpdateWebhookSubscriptionRequest, WebhookDelivery, WebhookSubscription,
};

use crate::{
    application::{
        composition::AppServices,
        docs::{BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE},
        errors::ApplicationError,
    },
    domains::account::{Permission, User},
    infra::axum::{Json, Path},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        create_webhook,
        list_webhooks,
        update_webhook,
        delete_webhook,
        rotate_webhook_secret,
        list_webhook_deliveries,
    ),
    components(schemas(
        CreateWebhookSubscriptionRequest,
        CreatedWebhookSubscription,
        UpdateWebhookSubscriptionRequest,
        WebhookSubscription,
        RotateWebhookSecretResponse,
        WebhookDelivery,
    )),
    tags((
        name = "Webhooks",
        description = "Signed, durable server-to-server notifications for account-owned wallets."
    ))
)]
pub struct WebhookHandler;

pub fn webhook_router() -> Router<Arc<AppServices>> {
    Router::new()
        .route(
            "/v1/me/wallets/{wallet_id}/webhooks",
            post(create_webhook).get(list_webhooks),
        )
        .route(
            "/v1/me/wallets/{wallet_id}/webhooks/{id}",
            put(update_webhook).delete(delete_webhook),
        )
        .route(
            "/v1/me/wallets/{wallet_id}/webhooks/{id}/rotate-secret",
            post(rotate_webhook_secret),
        )
        .route(
            "/v1/me/wallets/{wallet_id}/webhooks/{id}/deliveries",
            get(list_webhook_deliveries),
        )
}

#[utoipa::path(
    post,
    path = "/v1/me/wallets/{wallet_id}/webhooks",
    tag = "Webhooks",
    request_body = CreateWebhookSubscriptionRequest,
    responses(
        (status = 201, description = "Created; save the signing secret because it is returned only once", body = CreatedWebhookSubscription),
        (status = 400, description = "Invalid URL or event filter", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Wallet not found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE)),
    )
)]
async fn create_webhook(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Json(request): Json<CreateWebhookSubscriptionRequest>,
) -> Result<(StatusCode, Json<CreatedWebhookSubscription>), ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;
    Ok((
        StatusCode::CREATED,
        Json(services.webhook.create(user.account_id, wallet_id, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/v1/me/wallets/{wallet_id}/webhooks",
    tag = "Webhooks",
    responses(
        (status = 200, description = "Subscriptions", body = Vec<WebhookSubscription>),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Wallet not found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE)),
    )
)]
async fn list_webhooks(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
) -> Result<Json<Vec<WebhookSubscription>>, ApplicationError> {
    user.check_permission(Permission::ReadTransaction)?;
    Ok(Json(services.webhook.list(user.account_id, wallet_id).await?))
}

#[utoipa::path(
    put,
    path = "/v1/me/wallets/{wallet_id}/webhooks/{id}",
    tag = "Webhooks",
    request_body = UpdateWebhookSubscriptionRequest,
    responses(
        (status = 200, description = "Updated", body = WebhookSubscription),
        (status = 400, description = "Invalid URL or event filter", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Subscription not found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE)),
    )
)]
async fn update_webhook(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((wallet_id, id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateWebhookSubscriptionRequest>,
) -> Result<Json<WebhookSubscription>, ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;
    Ok(Json(
        services.webhook.update(user.account_id, wallet_id, id, request).await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/v1/me/wallets/{wallet_id}/webhooks/{id}",
    tag = "Webhooks",
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Subscription not found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE)),
    )
)]
async fn delete_webhook(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((wallet_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;
    services.webhook.delete(user.account_id, wallet_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/v1/me/wallets/{wallet_id}/webhooks/{id}/rotate-secret",
    tag = "Webhooks",
    responses(
        (status = 200, description = "Rotated; save the new secret because it is returned only once", body = RotateWebhookSecretResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Subscription not found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE)),
    )
)]
async fn rotate_webhook_secret(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((wallet_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RotateWebhookSecretResponse>, ApplicationError> {
    user.check_permission(Permission::WriteTransaction)?;
    Ok(Json(
        services.webhook.rotate_secret(user.account_id, wallet_id, id).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/v1/me/wallets/{wallet_id}/webhooks/{id}/deliveries",
    tag = "Webhooks",
    responses(
        (status = 200, description = "Newest 100 delivery records", body = Vec<WebhookDelivery>),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Subscription not found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE)),
    )
)]
async fn list_webhook_deliveries(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((wallet_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<WebhookDelivery>>, ApplicationError> {
    user.check_permission(Permission::ReadTransaction)?;
    Ok(Json(
        services.webhook.list_deliveries(user.account_id, wallet_id, id).await?,
    ))
}
