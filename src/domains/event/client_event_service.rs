use async_trait::async_trait;
use uuid::Uuid;

use crate::application::{composition::AppStore, errors::ApplicationError};

use super::{ClientEvent, ClientEventUseCases};

const EVENT_BATCH_SIZE: u64 = 100;

pub struct ClientEventService {
    store: AppStore,
}

impl ClientEventService {
    pub fn new(store: AppStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ClientEventUseCases for ClientEventService {
    async fn latest_id(&self, wallet_id: Uuid) -> Result<i32, ApplicationError> {
        Ok(self.store.client_event.latest_id(wallet_id).await?.unwrap_or_default())
    }

    async fn list_after(&self, wallet_id: Uuid, after_id: i32) -> Result<Vec<ClientEvent>, ApplicationError> {
        Ok(self
            .store
            .client_event
            .find_after(wallet_id, after_id, EVENT_BATCH_SIZE)
            .await?)
    }
}
