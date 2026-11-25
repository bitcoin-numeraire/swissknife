use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use swissknife_types::{SaveContactRequest, SavedContact};
use uuid::Uuid;

use super::account_wallet_handler::CONTEXT_PATH;
use crate::{
    application::{composition::AppServices, errors::ApplicationError},
    domains::agent::{WalletActor, WalletCapability},
    infra::axum::{Json, Path},
};

async fn verify_wallet(
    services: &AppServices,
    actor: WalletActor,
    wallet: Uuid,
    capability: WalletCapability,
) -> Result<(), ApplicationError> {
    let owner = actor.account_for(services, wallet, capability).await?;
    services.wallet.verify_ownership(owner, wallet).await
}

/// List saved Lightning Addresses for this wallet.
#[utoipa::path(get, path="/wallets/{wallet_id}/saved-contacts", context_path=CONTEXT_PATH, tag="Me",
    security(("jwt"=[]),("api_key"=[]),("wallet_key"=[])),
    responses((status=200, description="Saved contacts, ordered by address", body=Vec<SavedContact>),
    (status=401, description="Invalid credential"),(status=403,description="Read permission required"),
    (status=404,description="Wallet not found")))]
pub(super) async fn list_saved_contacts(
    State(services): State<Arc<AppServices>>,
    actor: WalletActor,
    Path(wallet_id): Path<Uuid>,
) -> Result<Json<Vec<SavedContact>>, ApplicationError> {
    verify_wallet(&services, actor, wallet_id, WalletCapability::Read).await?;
    Ok(Json(services.wallet.list_saved_contacts(wallet_id).await?))
}

/// Save a Lightning Address or update its label.
#[utoipa::path(post, path="/wallets/{wallet_id}/saved-contacts", context_path=CONTEXT_PATH, tag="Me",
    security(("jwt"=[]),("api_key"=[]),("wallet_key"=[])),request_body=SaveContactRequest,
    responses((status=200,description="Saved contact",body=SavedContact),
    (status=401,description="Invalid credential"),(status=403,description="Contact management permission required"),
    (status=404,description="Wallet not found"),(status=422,description="Invalid address or label")))]
pub(super) async fn save_contact(
    State(services): State<Arc<AppServices>>,
    actor: WalletActor,
    Path(wallet_id): Path<Uuid>,
    Json(request): Json<SaveContactRequest>,
) -> Result<Json<SavedContact>, ApplicationError> {
    verify_wallet(&services, actor, wallet_id, WalletCapability::ManageContacts).await?;
    Ok(Json(services.wallet.save_contact(wallet_id, request).await?))
}

/// Remove a saved contact. The wallet's payment history is retained.
#[utoipa::path(delete, path="/wallets/{wallet_id}/saved-contacts/{ln_address}", context_path=CONTEXT_PATH, tag="Me",
    security(("jwt"=[]),("api_key"=[]),("wallet_key"=[])),
    params(("wallet_id"=Uuid,Path,description="Wallet ID"),("ln_address"=String,Path,description="Saved Lightning Address")),
    responses((status=204,description="Saved contact removed"),(status=401,description="Invalid credential"),
    (status=403,description="Contact management permission required"),(status=404,description="Wallet or saved contact not found"),
    (status=422,description="Invalid Lightning Address")))]
pub(super) async fn remove_saved_contact(
    State(services): State<Arc<AppServices>>,
    actor: WalletActor,
    Path((wallet_id, ln_address)): Path<(Uuid, String)>,
) -> Result<StatusCode, ApplicationError> {
    verify_wallet(&services, actor, wallet_id, WalletCapability::ManageContacts).await?;
    services.wallet.remove_saved_contact(wallet_id, ln_address).await?;
    Ok(StatusCode::NO_CONTENT)
}
