use async_trait::async_trait;

use crate::application::errors::DatabaseError;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn ping(&self) -> Result<(), DatabaseError>;
}
