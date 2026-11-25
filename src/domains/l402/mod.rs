use std::num::NonZeroU32;

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

/// A paid invoice buys at most the configured number of successful resource responses.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RequestMeter: Send + Sync {
    async fn used(&self, payment_hash: [u8; 32]) -> Result<u64, DatabaseError>;
    async fn consume(
        &self,
        payment_hash: [u8; 32],
        wallet_id: Uuid,
        route: &str,
        limit: NonZeroU32,
    ) -> Result<bool, DatabaseError>;
}
