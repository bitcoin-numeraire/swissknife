use std::sync::Arc;

use axum::{extract::State, routing::get, Router};
use swissknife_types::{ErrorResponse, NodeOverview};
use utoipa::OpenApi;

use crate::{
    application::{composition::AppServices, errors::ApplicationError},
    domains::account::{Permission, User},
    infra::axum::Json,
};

#[derive(OpenApi)]
#[openapi(paths(read_node_overview), tags((name = "Node", description = "Lightning provider telemetry")))]
pub struct NodeHandler;

pub fn router() -> Router<Arc<AppServices>> {
    Router::new().route("/overview", get(read_node_overview))
}

/// Read live provider telemetry. Each unavailable section has an accompanying alert.
#[utoipa::path(get, path = "/v1/node/overview", tag = "Node", responses(
    (status = 200, description = "Current provider observations", body = NodeOverview),
    (status = 401, description = "Authentication required", body = ErrorResponse),
    (status = 403, description = "Requires read:ln_node", body = ErrorResponse)
))]
async fn read_node_overview(
    State(services): State<Arc<AppServices>>,
    user: User,
) -> Result<Json<NodeOverview>, ApplicationError> {
    user.check_permission(Permission::ReadLnNode)?;
    Ok(Json(services.node.overview().await))
}
