use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{bitcoin::BitcoinWallet, event::EventService},
};

#[async_trait]
pub trait EventsListener: Send + Sync {
    async fn listen(&self, events: EventService, bitcoin_wallet: Arc<dyn BitcoinWallet>) -> Result<(), LightningError>;
}
