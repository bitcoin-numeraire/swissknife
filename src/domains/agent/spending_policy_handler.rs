use crate::{
    application::{composition::AppServices, errors::ApplicationError},
    domains::account::{Permission, User},
    infra::axum::{Json, Path, Query},
};
use axum::{extract::State, http::StatusCode, routing::get, Router};
use std::sync::Arc;
use swissknife_types::{
    AgentSpendingHours, SpendingInterval, SpendingLimits, SpendingPolicy, SpendingReportQuery, SpendingReportRow,
    SpendingUsage,
};
use utoipa::OpenApi;
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(
        list_payment_approvals,
        get_payment_approval,
        decide_payment_approval,
        get_agent_spending_hours,
        set_agent_spending_hours,
        clear_agent_spending_hours,
        get_account_spending_policy,
        get_account_spending_report,
        export_account_spending_report,
        replace_account_spending_policy,
        disable_account_spending_policy
    ),
    components(schemas(
        swissknife_types::PaymentApproval,
        swissknife_types::ApprovalState,
        swissknife_types::ApprovalDecision,
        swissknife_types::DecidePaymentApproval,
        AgentSpendingHours,
        SpendingLimits,
        SpendingPolicy,
        SpendingUsage,
        SpendingReportRow,
        SpendingInterval
    ))
)]
pub struct SpendingPolicyHandler;

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route(
            "/v1/accounts/{account_id}/payment-approvals",
            get(list_payment_approvals),
        )
        .route(
            "/v1/accounts/{account_id}/payment-approvals/{id}",
            get(get_payment_approval).put(decide_payment_approval),
        )
        .route(
            "/v1/accounts/{account_id}/agent-spending-hours",
            get(get_agent_spending_hours)
                .put(set_agent_spending_hours)
                .delete(clear_agent_spending_hours),
        )
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

/// Inspect the UTC window for delegated payments.
#[utoipa::path(get,path="/v1/accounts/{account_id}/agent-spending-hours",tag="Accounts",security(("jwt"=[]),("api_key"=[])),params(("account_id"=Uuid,Path)),responses((status=200,body=AgentSpendingHours),(status=403),(status=404)))]
async fn get_agent_spending_hours(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(account): Path<Uuid>,
) -> Result<Json<AgentSpendingHours>, ApplicationError> {
    user.check_permission(Permission::ReadAccount)?;
    Ok(Json(services.spending_policies.hours(account).await?))
}

/// Restrict new payments authorized by a wallet credential to selected UTC weekdays and times.
#[utoipa::path(put,path="/v1/accounts/{account_id}/agent-spending-hours",tag="Accounts",security(("jwt"=[]),("api_key"=[])),params(("account_id"=Uuid,Path)),request_body=AgentSpendingHours,responses((status=204),(status=403),(status=404),(status=422)))]
async fn set_agent_spending_hours(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(account): Path<Uuid>,
    Json(hours): Json<AgentSpendingHours>,
) -> Result<StatusCode, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    services.account.get(account).await?;
    services.spending_policies.set_hours(account, Some(hours)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Remove the UTC admission window while preserving amount budgets and payment history.
#[utoipa::path(delete,path="/v1/accounts/{account_id}/agent-spending-hours",tag="Accounts",security(("jwt"=[]),("api_key"=[])),params(("account_id"=Uuid,Path)),responses((status=204),(status=403)))]
async fn clear_agent_spending_hours(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(account): Path<Uuid>,
) -> Result<StatusCode, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    services.spending_policies.set_hours(account, None).await?;
    Ok(StatusCode::NO_CONTENT)
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

/// List pending, unexpired delegated payment requests for an account.
#[utoipa::path(get,path="/v1/accounts/{account_id}/payment-approvals",tag="Accounts",security(("jwt"=[]),("api_key"=[])),params(("account_id"=Uuid,Path)),responses((status=200,body=Vec<swissknife_types::PaymentApproval>),(status=403),(status=404)))]
async fn list_payment_approvals(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(account): Path<Uuid>,
) -> Result<Json<Vec<swissknife_types::PaymentApproval>>, ApplicationError> {
    user.check_permission(Permission::ReadAccount)?;
    services.account.get(account).await?;
    Ok(Json(services.spending_policies.pending_approvals(account).await?))
}
/// Read the original destination, amount, fee ceiling and operator decision.
#[utoipa::path(get,path="/v1/accounts/{account_id}/payment-approvals/{id}",tag="Accounts",security(("jwt"=[]),("api_key"=[])),params(("account_id"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=swissknife_types::PaymentApproval),(status=403),(status=404)))]
async fn get_payment_approval(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((account, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<swissknife_types::PaymentApproval>, ApplicationError> {
    user.check_permission(Permission::ReadAccount)?;
    Ok(Json(services.spending_policies.approval(account, id).await?))
}
/// Approve one matching retry by the original credential or reject the request.
#[utoipa::path(put,path="/v1/accounts/{account_id}/payment-approvals/{id}",tag="Accounts",security(("jwt"=[]),("api_key"=[])),params(("account_id"=Uuid,Path),("id"=Uuid,Path)),request_body=swissknife_types::DecidePaymentApproval,responses((status=200,body=swissknife_types::PaymentApproval),(status=403),(status=404),(status=409)))]
async fn decide_payment_approval(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path((account, id)): Path<(Uuid, Uuid)>,
    Json(decision): Json<swissknife_types::DecidePaymentApproval>,
) -> Result<Json<swissknife_types::PaymentApproval>, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    Ok(Json(
        services
            .spending_policies
            .decide(account, id, user.account_id, decision.decision)
            .await?,
    ))
}
