use async_trait::async_trait;
use uuid::Uuid;

use crate::application::errors::DatabaseError;

use super::{Invoice, InvoiceFilter};

/// Public details of a verifiable invoice. The preimage is available only after settlement.
pub struct LnUrlInvoiceVerification {
    pub bolt11: String,
    pub settled_preimage: Option<String>,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_by_payment_hash(&self, payment_hash: &str) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_by_btc_output_id(&self, btc_output_id: Uuid) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_many(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, DatabaseError>;
    async fn insert(&self, invoice: Invoice) -> Result<Invoice, DatabaseError>;
    async fn insert_lnurl(&self, invoice: Invoice, verify_token: Uuid, preimage: [u8; 32])
        -> Result<(), DatabaseError>;
    async fn find_lnurl_verification(
        &self,
        verify_token: Uuid,
    ) -> Result<Option<LnUrlInvoiceVerification>, DatabaseError>;
    async fn update(&self, invoice: Invoice) -> Result<Invoice, DatabaseError>;
    /// Conditionally settle a still-pending invoice (sets payment_time/fee/received once).
    /// Returns `false` if the invoice was already settled, so callers stay idempotent.
    async fn settle(&self, invoice: &Invoice) -> Result<bool, DatabaseError>;
    async fn delete_many(&self, filter: InvoiceFilter) -> Result<u64, DatabaseError>;
}
