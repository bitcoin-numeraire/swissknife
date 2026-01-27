use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::{entities::EventsUseCases, errors::LightningError},
    domains::bitcoin::BitcoinWallet,
};

#[async_trait]
pub trait EventsListener: Send + Sync {
    async fn listen(&self, events: Arc<dyn EventsUseCases>, bitcoin_wallet: Arc<dyn BitcoinWallet>) -> Result<(), LightningError>;
}
