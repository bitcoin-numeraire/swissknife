use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;
use swissknife_types::{
    AccountNotification, ErrorResponse, NotificationAction, NotificationPage, NotificationQuery,
    ReadNotificationsRequest, UpdateNotificationRequest,
};
use utoipa::OpenApi;

use crate::{
    application::{composition::AppServices, errors::ApplicationError},
    domains::account::User,
    infra::axum::{Json, Path, Query},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        list_account_notifications,
        update_account_notification,
        mark_account_notifications_read
    ),
    components(schemas(
        AccountNotification,
        NotificationPage,
        NotificationQuery,
        NotificationAction,
        UpdateNotificationRequest,
        ReadNotificationsRequest
    ))
)]
pub struct NotificationHandler;

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", get(list_account_notifications))
        .route("/{id}", patch(update_account_notification))
        .route("/read", post(mark_account_notifications_read))
}

#[utoipa::path(get, path="/v1/me/notifications", tag="Account", params(NotificationQuery),
    responses((status=200,description="Newest account notifications and total unread count",body=NotificationPage),
              (status=401,description="Authentication required",body=ErrorResponse),
              (status=422,description="Invalid page or cursor",body=ErrorResponse)))]
async fn list_account_notifications(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query): Query<NotificationQuery>,
) -> Result<Json<NotificationPage>, ApplicationError> {
    Ok(Json(services.notification.page(user.account_id, query).await?))
}

#[utoipa::path(patch, path="/v1/me/notifications/{id}", tag="Account", params(("id"=i32,Path,description="Source event ID")),
    request_body=UpdateNotificationRequest, responses((status=204,description="Notification updated"),
    (status=401,description="Authentication required",body=ErrorResponse),(status=404,description="Notification not found in this account",body=ErrorResponse)))]
async fn update_account_notification(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<i32>,
    Json(body): Json<UpdateNotificationRequest>,
) -> Result<StatusCode, ApplicationError> {
    services.notification.update(user.account_id, id, body.action).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path="/v1/me/notifications/read", tag="Account", request_body=ReadNotificationsRequest,
    responses((status=204,description="Account notifications marked read through the supplied ID"),
    (status=401,description="Authentication required",body=ErrorResponse),(status=422,description="Invalid cursor",body=ErrorResponse)))]
async fn mark_account_notifications_read(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(body): Json<ReadNotificationsRequest>,
) -> Result<StatusCode, ApplicationError> {
    services
        .notification
        .read_through(user.account_id, body.through)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
