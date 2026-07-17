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
        docs::{BAD_REQUEST_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE, UNAUTHORIZED_EXAMPLE},
        errors::{ApplicationError, DataError},
    },
    domains::account::{Permission, User},
    infra::axum::{Path, Query},
};

const LAST_EVENT_ID: &str = "last-event-id";
const EVENT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ClientEventStreamQuery {
    /// Replay events strictly after this event ID. `Last-Event-ID` takes precedence.
    after: Option<i32>,
}

#[derive(OpenApi)]
#[openapi(
    paths(stream_wallet_events),
    components(schemas(ClientEvent, ClientEventType)),
    tags((name = "Events", description = "Authenticated, wallet-scoped server-sent events."))
)]
pub struct ClientEventHandler;

pub fn client_event_router() -> Router<Arc<AppServices>> {
    Router::new().route("/v1/me/wallets/{wallet_id}/events", get(stream_wallet_events))
}

/// Stream durable wallet events.
///
/// A fresh connection starts after the latest committed event. Send `Last-Event-ID`
/// on reconnect (or `after` for a deliberate replay) to receive missed events.
#[utoipa::path(
    get,
    path = "/v1/me/wallets/{wallet_id}/events",
    tag = "Events",
    params(("wallet_id" = Uuid, Path, description = "Account-owned wallet ID"), ClientEventStreamQuery),
    responses(
        (status = 200, description = "Server-sent event stream", body = ClientEvent, content_type = "text/event-stream"),
        (status = 400, description = "Invalid replay cursor", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Wallet not found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn stream_wallet_events(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(wallet_id): Path<Uuid>,
    Query(query): Query<ClientEventStreamQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApplicationError> {
    user.check_permission(Permission::ReadTransaction)?;
    services.wallet.verify_ownership(user.account_id, wallet_id).await?;

    let cursor = match replay_cursor(&headers, query.after)? {
        Some(cursor) => cursor,
        None => services.client_event.latest_id(wallet_id).await?,
    };

    let stream = wallet_event_stream(services, wallet_id, cursor);
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
        return Ok(query_after);
    };

    let raw = raw
        .to_str()
        .map_err(|_| DataError::Malformed("Last-Event-ID must be a positive integer.".to_string()))?;
    let cursor = raw
        .parse::<i32>()
        .map_err(|_| DataError::Malformed("Last-Event-ID must be a positive integer.".to_string()))?;
    if cursor < 0 {
        return Err(DataError::Malformed("Last-Event-ID must be a positive integer.".to_string()).into());
    }

    Ok(Some(cursor))
}

struct EventStreamState {
    services: Arc<AppServices>,
    wallet_id: Uuid,
    cursor: i32,
    pending: VecDeque<ClientEvent>,
}

fn wallet_event_stream(
    services: Arc<AppServices>,
    wallet_id: Uuid,
    cursor: i32,
) -> impl Stream<Item = Result<Event, Infallible>> {
    stream::unfold(
        EventStreamState {
            services,
            wallet_id,
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
                    .list_after(state.wallet_id, state.cursor)
                    .await
                {
                    Ok(events) if !events.is_empty() => state.pending.extend(events),
                    Ok(_) => tokio::time::sleep(EVENT_POLL_INTERVAL).await,
                    Err(error) => {
                        warn!(%error, wallet_id = %state.wallet_id, "Failed to read the client event log");
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

    #[tokio::test]
    async fn requires_transaction_read_permission() {
        let services = MockAppServicesBuilder::new().build();

        let result = stream_wallet_events(
            State(Arc::new(services)),
            user(Uuid::new_v4(), vec![]),
            Path(Uuid::new_v4()),
            Query(ClientEventStreamQuery { after: None }),
            HeaderMap::new(),
        )
        .await;

        assert!(matches!(result, Err(ApplicationError::Authorization(_))));
    }

    #[tokio::test]
    async fn verifies_wallet_ownership_and_starts_fresh_at_the_latest_event() {
        let account_id = Uuid::new_v4();
        let wallet_id = Uuid::new_v4();
        let mut services = MockAppServicesBuilder::new();
        services
            .wallet
            .expect_verify_ownership()
            .withf(move |account, wallet| *account == account_id && *wallet == wallet_id)
            .times(1)
            .returning(|_, _| Ok(()));
        services
            .client_event
            .expect_latest_id()
            .withf(move |wallet| *wallet == wallet_id)
            .times(1)
            .returning(|_| Ok(42));

        let result = stream_wallet_events(
            State(Arc::new(services.build())),
            user(account_id, vec![Permission::ReadTransaction]),
            Path(wallet_id),
            Query(ClientEventStreamQuery { after: None }),
            HeaderMap::new(),
        )
        .await;

        assert!(result.is_ok());
    }
}
