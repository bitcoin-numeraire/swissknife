use async_trait::async_trait;

use super::Asset;
use crate::application::errors::ApplicationError;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AssetUseCases: Send + Sync {
    async fn list(&self) -> Result<Vec<Asset>, ApplicationError>;
}
