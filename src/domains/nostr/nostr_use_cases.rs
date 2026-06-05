use async_trait::async_trait;
use nostr_sdk::PublicKey;

use crate::application::errors::ApplicationError;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait NostrUseCases: Send + Sync {
    async fn get_pubkey(&self, username: String) -> Result<PublicKey, ApplicationError>;
}
