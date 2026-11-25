use crate::{
    application::{composition::AppServices, errors::ApplicationError},
    domains::account::{Permission, User},
    infra::axum::{Json, Path},
};
use axum::{extract::State, http::StatusCode, routing::get, Router};
use std::sync::Arc;
use swissknife_types::{SpendingLimits, SpendingPolicy, SpendingUsage};
use utoipa::OpenApi;
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(
        get_account_spending_policy,
        replace_account_spending_policy,
        disable_account_spending_policy
    ),
    components(schemas(SpendingLimits, SpendingPolicy, SpendingUsage))
)]
pub struct SpendingPolicyHandler;

pub fn router() -> Router<Arc<AppServices>> {
    Router::new().route(
        "/v1/accounts/{account_id}/spending-policy",
        get(get_account_spending_policy)
            .put(replace_account_spending_policy)
            .delete(disable_account_spending_policy),
    )
}

/// Read the active spending limits and current calendar-period usage.
#[utoipa::path(get, path="/v1/accounts/{account_id}/spending-policy", tag="Accounts", security(("jwt"=[]),("api_key"=[])), params(("account_id"=Uuid,Path)), responses((status=200,body=SpendingPolicy),(status=403),(status=404)))]
async fn get_account_spending_policy(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(account): Path<Uuid>,
) -> Result<Json<SpendingPolicy>, ApplicationError> {
    user.check_permission(Permission::ReadAccount)?;
    Ok(Json(services.spending_policies.get(account).await?))
}

/// Apply limits to future outgoing payments from every wallet owned by the account.
#[utoipa::path(put, path="/v1/accounts/{account_id}/spending-policy", tag="Accounts", security(("jwt"=[]),("api_key"=[])), params(("account_id"=Uuid,Path)), request_body=SpendingLimits, responses((status=204),(status=403),(status=404),(status=422)))]
async fn replace_account_spending_policy(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(account): Path<Uuid>,
    Json(limits): Json<SpendingLimits>,
) -> Result<StatusCode, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    services.account.get(account).await?;
    services.spending_policies.replace(account, limits).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Disable enforcement while retaining consumption already admitted under the policy.
#[utoipa::path(delete, path="/v1/accounts/{account_id}/spending-policy", tag="Accounts", security(("jwt"=[]),("api_key"=[])), params(("account_id"=Uuid,Path)), responses((status=204),(status=403)))]
async fn disable_account_spending_policy(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(account): Path<Uuid>,
) -> Result<StatusCode, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    services.spending_policies.disable(account).await?;
    Ok(StatusCode::NO_CONTENT)
}
