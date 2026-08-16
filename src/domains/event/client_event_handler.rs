use std::{collections::VecDeque, convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::State,
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue},
    response::{sse::Event, IntoResponse, Sse},
    routing::get,
    Router,
};
use futures_util::stream::{self, Stream};
use serde::Deserialize;
use tracing::warn;
use utoipa::{IntoParams, OpenApi};
use uuid::Uuid;

use swissknife_types::{ClientEvent, ClientEventType, ErrorResponse};

use crate::{
    application::{
        composition::AppServices,
        docs::{BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, UNAUTHORIZED_EXAMPLE},
        errors::{ApplicationError, DataError},
    },
    domains::account::{Permission, User},
    infra::axum::Query,
};

const LAST_EVENT_ID: &str = "last-event-id";
const EVENT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);
const CURSOR_EXPIRED_EXAMPLE: &str = r#"
{
    "status": "409 Conflict",
    "reason": "Client event cursor expired. Refresh state and reconnect without Last-Event-ID."
}
"#;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ClientEventStreamQuery {
    /// Replay events strictly after this event ID. `Last-Event-ID` takes precedence.
    after: Option<i32>,
}

#[derive(OpenApi)]
#[openapi(
    paths(stream_account_events),
    components(schemas(ClientEvent, ClientEventType)),
    tags((name = "Events", description = "Authenticated, account-scoped server-sent events."))
)]
pub struct ClientEventHandler;

pub fn client_event_router() -> Router<Arc<AppServices>> {
    Router::new().route("/v1/me/events", get(stream_account_events))
}

/// Stream durable events for every wallet owned by the authenticated account.
///
/// A fresh connection starts after the latest committed account event. Send
/// `Last-Event-ID` on reconnect (or `after` for deliberate replay) to receive
/// missed events. If that cursor is older than the retained replay window,
/// refresh REST state and reconnect without a cursor.
#[utoipa::path(
    get,
    path = "/v1/me/events",
    tag = "Events",
    params(ClientEventStreamQuery),
    responses(
        (status = 200, description = "Server-sent event stream", body = ClientEvent, content_type = "text/event-stream"),
        (status = 400, description = "Invalid replay cursor", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 409, description = "Replay cursor has expired", body = ErrorResponse, example = json!(CURSOR_EXPIRED_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn stream_account_events(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(query): Query<ClientEventStreamQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApplicationError> {
    user.check_permission(Permission::ReadTransaction)?;

    let cursor = match replay_cursor(&headers, query.after)? {
        Some(cursor) => {
            services.client_event.ensure_cursor_available(cursor).await?;
            cursor
        }
        None => services.client_event.latest_id(user.account_id).await?,
    };

    let stream = account_event_stream(services, user.account_id, cursor);
    let sse = Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(KEEP_ALIVE_INTERVAL)
            .text("keep-alive"),
    );

    Ok((
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-cache, no-transform")),
            (
                axum::http::HeaderName::from_static("x-accel-buffering"),
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
}

fn account_event_stream(
    services: Arc<AppServices>,
    account_id: Uuid,
    cursor: i32,
) -> impl Stream<Item = Result<Event, Infallible>> {
    stream::unfold(
        EventStreamState {
            services,
            account_id,
            cursor,
            pending: VecDeque::new(),
        },
        |mut state| async move {
            loop {
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
                    Ok(_) => tokio::time::sleep(EVENT_POLL_INTERVAL).await,
                    Err(ApplicationError::Data(DataError::Conflict(error))) => {
                        warn!(%error, account_id = %state.account_id, "Client event cursor expired while streaming");
                        return None;
                    }
                    Err(error) => {
                        warn!(%error, account_id = %state.account_id, "Failed to read the client event log");
                        tokio::time::sleep(EVENT_POLL_INTERVAL).await;
                    }
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::application::composition::MockAppServicesBuilder;

    use super::*;

    fn user(account_id: Uuid, permissions: Vec<Permission>) -> User {
        User {
            account_id,
            permissions,
        }
    }

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

    #[tokio::test]
    async fn requires_transaction_read_permission() {
        let services = MockAppServicesBuilder::new().build();

        let result = stream_account_events(
            State(Arc::new(services)),
            user(Uuid::new_v4(), vec![]),
            Query(ClientEventStreamQuery { after: None }),
            HeaderMap::new(),
        )
        .await;

        assert!(matches!(result, Err(ApplicationError::Authorization(_))));
    }

    #[tokio::test]
    async fn fresh_stream_starts_at_the_accounts_latest_event() {
        let account_id = Uuid::new_v4();
        let mut services = MockAppServicesBuilder::new();
        services
            .client_event
            .expect_latest_id()
            .withf(move |account| *account == account_id)
            .times(1)
            .returning(|_| Ok(42));

        let result = stream_account_events(
            State(Arc::new(services.build())),
            user(account_id, vec![Permission::ReadTransaction]),
            Query(ClientEventStreamQuery { after: None }),
            HeaderMap::new(),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn explicit_replay_validates_the_cursor_before_opening() {
        let account_id = Uuid::new_v4();
        let mut services = MockAppServicesBuilder::new();
        services
            .client_event
            .expect_ensure_cursor_available()
            .withf(|cursor| *cursor == 42)
            .times(1)
            .returning(|_| Ok(()));

        let result = stream_account_events(
            State(Arc::new(services.build())),
            user(account_id, vec![Permission::ReadTransaction]),
            Query(ClientEventStreamQuery { after: Some(42) }),
            HeaderMap::new(),
        )
        .await;

        assert!(result.is_ok());
    }
}
