use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{bitcoin::{BitcoinEventsUseCases, BitcoinWallet}, ln_node::LnEventsUseCases},
};

#[async_trait]
pub trait LnNodeListener: Send + Sync {
    async fn listen(
        &self,
        ln_events: Arc<dyn LnEventsUseCases>,
        bitcoin_events: Arc<dyn BitcoinEventsUseCases>,
        bitcoin_wallet: Arc<dyn BitcoinWallet>,
    ) -> Result<(), LightningError>;
}
