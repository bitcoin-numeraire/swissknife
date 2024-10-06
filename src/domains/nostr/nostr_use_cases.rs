use async_trait::async_trait;
use nostr_sdk::PublicKey;

use crate::application::errors::ApplicationError;

#[async_trait]
pub trait NostrUseCases: Send + Sync {
    async fn get_pubkey(&self, username: String) -> Result<PublicKey, ApplicationError>;
}
