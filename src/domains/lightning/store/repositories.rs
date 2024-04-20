use async_trait::async_trait;
use sqlx::{Postgres, Transaction};

use crate::{
    application::errors::DatabaseError,
    domains::lightning::entities::{
        LightningAddress, LightningInvoice, LightningPayment, UserBalance,
    },
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
    async fn get_balance_by_username(
        &self,
        executor: Option<&mut Transaction<'_, Postgres>>,
        username: &str,
    ) -> Result<UserBalance, DatabaseError>;
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

#[async_trait]
pub trait LightningPaymentRepository: Sync + Send {
    async fn get_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningPayment>, DatabaseError>;
    async fn insert(
        &self,
        executor: Option<&mut Transaction<'_, Postgres>>,
        payment: LightningPayment,
    ) -> Result<LightningPayment, DatabaseError>;
    async fn update(&self, payment: LightningPayment) -> Result<LightningPayment, DatabaseError>;
}
