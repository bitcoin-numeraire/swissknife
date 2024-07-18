use async_trait::async_trait;

use crate::application::errors::ApplicationError;

use super::{LnURLPayRequest, LnUrlCallback};

#[async_trait]
pub trait LnUrlUseCases: Send + Sync {
    async fn lnurlp(&self, username: String) -> Result<LnURLPayRequest, ApplicationError>;
    async fn lnurlp_callback(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<LnUrlCallback, ApplicationError>;
}
