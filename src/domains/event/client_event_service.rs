use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError},
};

use super::{ClientEvent, ClientEventUseCases};

const EVENT_BATCH_SIZE: u64 = 100;
const EXPIRED_CURSOR_MESSAGE: &str = "Client event cursor expired. Refresh state and reconnect without Last-Event-ID.";

pub struct ClientEventService {
    store: AppStore,
    retention: Duration,
}

impl ClientEventService {
    pub fn new(store: AppStore, retention: Duration) -> Self {
        Self { store, retention }
    }

    fn retention_cutoff(&self) -> Result<Option<DateTime<Utc>>, ApplicationError> {
        if self.retention.is_zero() {
            return Ok(None);
        }

        let retention = chrono::Duration::from_std(self.retention)
            .map_err(|error| DataError::Validation(format!("Invalid client event retention: {error}")))?;
        Ok(Some(Utc::now() - retention))
    }
}

#[async_trait]
impl ClientEventUseCases for ClientEventService {
    async fn latest_id(&self, account_id: Uuid) -> Result<i32, ApplicationError> {
        let latest_id = self.store.client_event.latest_id(account_id).await?.unwrap_or_default();
        let pruned_through = self.store.client_event.pruned_through().await?;

        // A fresh stream for an account with no retained events must begin at
        // least after the global prune watermark. Otherwise its initial cursor
        // would immediately look like an expired replay request.
        Ok(latest_id.max(pruned_through))
    }

    async fn ensure_cursor_available(&self, after_id: i32) -> Result<(), ApplicationError> {
        let pruned_through = self.store.client_event.pruned_through().await?;
        if pruned_through > 0 && after_id <= pruned_through {
            return Err(DataError::Conflict(EXPIRED_CURSOR_MESSAGE.to_string()).into());
        }

        Ok(())
    }

    async fn list_after(&self, account_id: Uuid, after_id: i32) -> Result<Vec<ClientEvent>, ApplicationError> {
        self.ensure_cursor_available(after_id).await?;
        let events = self
            .store
            .client_event
            .find_after(account_id, after_id, EVENT_BATCH_SIZE)
            .await?;
        // Close the race where pruning commits between the first watermark
        // check and the event query. A conflict is conservative; returning a
        // partial batch without telling the client to refresh is not.
        self.ensure_cursor_available(after_id).await?;

        Ok(events)
    }

    async fn prune(&self) -> Result<u64, ApplicationError> {
        let Some(cutoff) = self.retention_cutoff()? else {
            return Ok(0);
        };

        Ok(self.store.client_event.prune_before(cutoff).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;

    use crate::application::composition::MockAppStoreBuilder;

    use super::*;

    #[tokio::test]
    async fn rejects_a_cursor_at_or_before_the_pruned_watermark() {
        let mut store = MockAppStoreBuilder::new();
        store.client_event.expect_pruned_through().return_once(|| Ok(42));
        let service = ClientEventService::new(store.build(), Duration::from_secs(60));

        let error = service.ensure_cursor_available(42).await.unwrap_err();

        assert!(matches!(error, ApplicationError::Data(DataError::Conflict(_))));
    }

    #[tokio::test]
    async fn lists_only_events_for_the_authenticated_account() {
        let account_id = Uuid::new_v4();
        let mut store = MockAppStoreBuilder::new();
        store.client_event.expect_pruned_through().times(2).returning(|| Ok(3));
        store
            .client_event
            .expect_find_after()
            .withf(move |account, after, limit| *account == account_id && *after == 7 && *limit == EVENT_BATCH_SIZE)
            .return_once(|_, _, _| Ok(vec![]));
        let service = ClientEventService::new(store.build(), Duration::from_secs(60));

        assert!(service.list_after(account_id, 7).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn fresh_cursor_never_precedes_the_prune_watermark() {
        let account_id = Uuid::new_v4();
        let mut store = MockAppStoreBuilder::new();
        store
            .client_event
            .expect_latest_id()
            .withf(move |account| *account == account_id)
            .return_once(|_| Ok(None));
        store.client_event.expect_pruned_through().return_once(|| Ok(42));
        let service = ClientEventService::new(store.build(), Duration::from_secs(60));

        assert_eq!(service.latest_id(account_id).await.unwrap(), 42);
    }

    #[tokio::test]
    async fn pruning_uses_the_configured_retention_window() {
        let before = Utc::now() - chrono::Duration::hours(1);
        let mut store = MockAppStoreBuilder::new();
        store
            .client_event
            .expect_prune_before()
            .withf(move |cutoff| *cutoff >= before && *cutoff <= Utc::now())
            .return_once(|_| Ok(4));
        let service = ClientEventService::new(store.build(), Duration::from_secs(60 * 60));

        assert_eq!(service.prune().await.unwrap(), 4);
    }

    #[tokio::test]
    async fn zero_retention_disables_pruning() {
        let store = MockAppStoreBuilder::new();
        let service = ClientEventService::new(store.build(), Duration::ZERO);

        assert_eq!(service.prune().await.unwrap(), 0);
    }
}
