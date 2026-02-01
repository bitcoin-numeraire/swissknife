use async_trait::async_trait;

use crate::application::errors::LightningError;

#[async_trait]
pub trait EventsListener: Send + Sync {
    async fn listen(&self) -> Result<(), LightningError>;
}
