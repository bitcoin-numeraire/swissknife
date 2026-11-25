use crate::{
    application::{composition::AppServices, errors::ApplicationError},
    domains::account::{Permission, User},
    infra::axum::{Json, Path, Query},
};
use axum::{extract::State, http::StatusCode, routing::get, Router};
use std::sync::Arc;
use swissknife_types::{
    SpendingInterval, SpendingLimits, SpendingPolicy, SpendingReportQuery, SpendingReportRow, SpendingUsage,
};
use utoipa::OpenApi;
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(
        get_account_spending_policy,
        get_account_spending_report,
        export_account_spending_report,
        replace_account_spending_policy,
        disable_account_spending_policy
    ),
    components(schemas(SpendingLimits, SpendingPolicy, SpendingUsage, SpendingReportRow, SpendingInterval))
)]
pub struct SpendingPolicyHandler;

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route(
            "/v1/accounts/{account_id}/spending-report",
            get(get_account_spending_report),
        )
        .route(
            "/v1/accounts/{account_id}/spending-report.csv",
            get(export_account_spending_report),
        )
        .route(
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

/// Read current budget consumption grouped by the day or week of original admission.
#[utoipa::path(get,path="/v1/accounts/{account_id}/spending-report",tag="Accounts",security(("jwt"=[]),("api_key"=[])),
    params(("account_id"=Uuid,Path),SpendingReportQuery), responses((status=200,body=Vec<SpendingReportRow>),(status=403),(status=404),(status=422)))]
async fn get_account_spending_report(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(account): Path<Uuid>,
    Query(query): Query<SpendingReportQuery>,
) -> Result<Json<Vec<SpendingReportRow>>, ApplicationError> {
    user.check_permission(Permission::ReadAccount)?;
    services.account.get(account).await?;
    Ok(Json(services.spending_policies.report(account, query).await?))
}

/// Download the same spending report as a CSV with exact integer millisatoshis.
#[utoipa::path(get,path="/v1/accounts/{account_id}/spending-report.csv",tag="Accounts",security(("jwt"=[]),("api_key"=[])),
    params(("account_id"=Uuid,Path),SpendingReportQuery), responses((status=200,body=String,content_type="text/csv"),(status=403),(status=404),(status=422)))]
async fn export_account_spending_report(
    state: State<Arc<AppServices>>,
    user: User,
    account: Path<Uuid>,
    query: Query<SpendingReportQuery>,
) -> Result<impl axum::response::IntoResponse, ApplicationError> {
    let Json(rows) = get_account_spending_report(state, user, account, query).await?;
    use axum::http::header;
    let mut csv = String::from("period_start,payments,charged_msat\r\n");
    for row in rows {
        csv.push_str(&format!(
            "{},{},{}\r\n",
            row.period_start, row.payments, row.charged_msat
        ));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=spending-report.csv"),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
        csv,
    ))
}
