use super::{CreateOfferRequest, OfferFilter, OfferState, WalletOffer};
use crate::application::errors::ApplicationError;
use async_trait::async_trait;
use serde::Deserialize;
use uuid::Uuid;

pub struct StoredOffer {
    pub offer: WalletOffer,
    pub backend: String,
}
#[derive(Debug, Deserialize)]
pub struct NodeOffer {
    pub offer_id: String,
    pub bolt12: String,
    pub active: bool,
    pub label: Option<String>,
}

/// Keep the provider index for every update, including unrelated and unpaid invoices.
#[derive(Debug, Deserialize)]
pub struct OfferUpdate {
    pub updated_index: u64,
    pub status: String,
    pub local_offer_id: Option<String>,
    pub payment_hash: String,
    pub bolt12: Option<String>,
    pub amount_msat: Option<u64>,
    pub amount_received_msat: Option<u64>,
    pub paid_at: Option<u64>,
    #[serde(skip)]
    pub invoice_created_at: Option<u64>,
    pub invreq_payer_note: Option<String>,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OfferRepository: Send + Sync {
    async fn register(
        &self,
        wallet: Uuid,
        backend: &str,
        request: &CreateOfferRequest,
    ) -> Result<(StoredOffer, bool), ApplicationError>;
    async fn owner(&self, provider_id: &str) -> Result<Option<Uuid>, ApplicationError>;
    async fn get(&self, id: Uuid, wallet: Option<Uuid>) -> Result<StoredOffer, ApplicationError>;
    async fn list(&self, filter: OfferFilter) -> Result<Vec<WalletOffer>, ApplicationError>;
    async fn activate(&self, id: Uuid, node: &NodeOffer) -> Result<StoredOffer, ApplicationError>;
    async fn transition(&self, id: Uuid, from: OfferState, to: OfferState) -> Result<StoredOffer, ApplicationError>;
    /// Recover registrations and disables interrupted before the provider reply was recorded.
    async fn pending(&self, backend: &str) -> Result<Option<StoredOffer>, ApplicationError>;
    async fn cursor(&self, backend: &str) -> Result<Option<u64>, ApplicationError>;
    async fn project(&self, backend: &str, after: u64, updates: Vec<OfferUpdate>) -> Result<(), ApplicationError>;
}
