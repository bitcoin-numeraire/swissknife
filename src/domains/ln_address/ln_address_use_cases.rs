use async_trait::async_trait;

use nostr_sdk::PublicKey;
use uuid::Uuid;

use swissknife_types::UpdateLnAddressRequest;

use crate::application::errors::ApplicationError;

use super::{LnAddress, LnAddressFilter};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LnAddressUseCases: Send + Sync {
    async fn register(
        &self,
        wallet_id: Uuid,
        username: String,
        allows_nostr: bool,
        nostr_pubkey: Option<PublicKey>,
    ) -> Result<LnAddress, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<LnAddress, ApplicationError>;
    async fn list(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, ApplicationError>;
    async fn update(&self, id: Uuid, request: UpdateLnAddressRequest) -> Result<LnAddress, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: LnAddressFilter) -> Result<u64, ApplicationError>;
}
