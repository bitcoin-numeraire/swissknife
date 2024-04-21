use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::{
    application::errors::DatabaseError,
    domains::lightning::entities::{
        LightningAddress, LightningInvoice, LightningPayment, UserBalance,
    },
};

#[async_trait]
pub trait LightningAddressRepository: Sync + Send {
    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_by_user_id(&self, user: &str) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_all(
        &self,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn find_all_by_user_id(
        &self,
        user: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn insert(&self, user: &str, username: &str) -> Result<LightningAddress, DatabaseError>;
    async fn get_balance_by_username(&self, username: &str) -> Result<UserBalance, DatabaseError>;
}

#[async_trait]
pub trait LightningInvoiceRepository: Sync + Send {
    async fn find_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningInvoice>, DatabaseError>;
    async fn insert(&self, invoice: LightningInvoice) -> Result<LightningInvoice, DatabaseError>;
    async fn update(&self, invoice: LightningInvoice) -> Result<LightningInvoice, DatabaseError>;
}

#[async_trait]
pub trait LightningPaymentRepository: Sync + Send {
    async fn find_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningPayment>, DatabaseError>;
    async fn insert(
        &self,
        executor: &DatabaseConnection,
        payment: LightningPayment,
    ) -> Result<LightningPayment, DatabaseError>;
    async fn update(&self, payment: LightningPayment) -> Result<LightningPayment, DatabaseError>;
}
