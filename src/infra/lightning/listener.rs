use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{bitcoin::BitcoinWallet, event::EventUseCases},
};

#[async_trait]
pub trait EventsListener: Send + Sync {
    async fn listen(
        &self,
        events: Arc<dyn EventUseCases>,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError>;
}
