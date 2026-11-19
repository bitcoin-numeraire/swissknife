use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::ApplicationError;
use swissknife_types::LnUrlVerifyResponse;

use super::{LnURLPayRequest, LnUrlCallback};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LnUrlUseCases: Send + Sync {
    async fn lnurlp(&self, username: String) -> Result<LnURLPayRequest, ApplicationError>;
    async fn lnurlp_callback(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<LnUrlCallback, ApplicationError>;
    async fn verify(&self, token: Uuid) -> Result<LnUrlVerifyResponse, ApplicationError>;
}
