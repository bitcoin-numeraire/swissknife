use crate::{
    application::{composition::AppServices, errors::ApplicationError},
    domains::account::User,
    infra::axum::{Json, Path},
};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, get},
    Router,
};
use std::sync::Arc;
use swissknife_types::{AgentKey, CreateAgentKey, IssuedAgentKey, WalletCapabilities};
use utoipa::OpenApi;
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(paths(issue, list, revoke), components(schemas(AgentKey, CreateAgentKey, IssuedAgentKey, WalletCapabilities)), tags((name="Agent credentials",description="Owner-managed access to a single wallet.")))]
pub struct AgentKeyHandler;

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/v1/me/wallets/{wallet_id}/agent-keys", get(list).post(issue))
        .route("/v1/me/wallets/{wallet_id}/agent-keys/{key_id}", delete(revoke))
}

#[utoipa::path(post,path="/v1/me/wallets/{wallet_id}/agent-keys",tag="Agent credentials",security(("jwt"=[]),("api_key"=[])),request_body=CreateAgentKey,params(("wallet_id"=Uuid,Path)),responses((status=201,body=IssuedAgentKey),(status=404),(status=422)))]
async fn issue(
    State(services): State<Arc<AppServices>>,
    owner: User,
    Path(wallet): Path<Uuid>,
    Json(request): Json<CreateAgentKey>,
) -> Result<(StatusCode, Json<IssuedAgentKey>), ApplicationError> {
    services.wallet.verify_ownership(owner.account_id, wallet).await?;
    Ok((
        StatusCode::CREATED,
        Json(services.agent_keys.issue(owner.account_id, wallet, request).await?),
    ))
}

#[utoipa::path(get,path="/v1/me/wallets/{wallet_id}/agent-keys",tag="Agent credentials",security(("jwt"=[]),("api_key"=[])),params(("wallet_id"=Uuid,Path)),responses((status=200,body=Vec<AgentKey>),(status=404)))]
async fn list(
    State(services): State<Arc<AppServices>>,
    owner: User,
    Path(wallet): Path<Uuid>,
) -> Result<Json<Vec<AgentKey>>, ApplicationError> {
    services.wallet.verify_ownership(owner.account_id, wallet).await?;
    Ok(Json(services.agent_keys.list(owner.account_id, wallet).await?))
}

#[utoipa::path(delete,path="/v1/me/wallets/{wallet_id}/agent-keys/{key_id}",tag="Agent credentials",security(("jwt"=[]),("api_key"=[])),params(("wallet_id"=Uuid,Path),("key_id"=Uuid,Path)),responses((status=204),(status=404)))]
async fn revoke(
    State(services): State<Arc<AppServices>>,
    owner: User,
    Path((wallet, key)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApplicationError> {
    services.wallet.verify_ownership(owner.account_id, wallet).await?;
    services.agent_keys.revoke(owner.account_id, wallet, key).await?;
    Ok(StatusCode::NO_CONTENT)
}
