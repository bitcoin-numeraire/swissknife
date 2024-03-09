use async_trait::async_trait;

use crate::{
    application::errors::DatabaseError,
    domains::lightning::entities::{LightningAddress, LightningInvoice},
};

#[async_trait]
pub trait LightningAddressRepository: Sync + Send {
    async fn get_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn get_by_user_id(&self, user: &str) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn list(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn list_by_user_id(
        &self,
        user: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;

    async fn insert(&self, user: &str, username: &str) -> Result<LightningAddress, DatabaseError>;
}
#[async_trait]
pub trait LightningInvoiceRepository: Sync + Send {
    async fn get_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningInvoice>, DatabaseError>;
    async fn insert(&self, invoice: LightningInvoice) -> Result<LightningInvoice, DatabaseError>;
    async fn update(&self, invoice: LightningInvoice) -> Result<LightningInvoice, DatabaseError>;
}

pub struct LightningStore {
    pub address: Box<dyn LightningAddressRepository>,
    pub invoice: Box<dyn LightningInvoiceRepository>,
}

impl LightningStore {
    pub fn new(
        address: Box<dyn LightningAddressRepository>,
        invoice: Box<dyn LightningInvoiceRepository>,
    ) -> Self {
        LightningStore { address, invoice }
    }
}
