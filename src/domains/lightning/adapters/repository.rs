use async_trait::async_trait;

use crate::{
    application::errors::DatabaseError,
    domains::lightning::entities::{
        LightningAddress, LightningInvoice, LightningPayment, UserBalance,
    },
};

#[async_trait]
pub trait LightningAddressRepository: Sync + Send {
    async fn find_address_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_address_by_user_id(
        &self,
        user: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_all_addresses(
        &self,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn find_all_addresses_by_user_id(
        &self,
        user: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn insert_address(
        &self,
        user: &str,
        username: &str,
    ) -> Result<LightningAddress, DatabaseError>;
    async fn get_balance_by_username(&self, username: &str) -> Result<UserBalance, DatabaseError>;
}

#[async_trait]
pub trait LightningInvoiceRepository: Sync + Send {
    async fn find_invoice_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningInvoice>, DatabaseError>;
    async fn insert_invoice(
        &self,
        invoice: LightningInvoice,
    ) -> Result<LightningInvoice, DatabaseError>;
    async fn update_invoice(
        &self,
        invoice: LightningInvoice,
    ) -> Result<LightningInvoice, DatabaseError>;
}

#[async_trait]
pub trait LightningPaymentRepository: Sync + Send {
    async fn find_payment_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningPayment>, DatabaseError>;
    async fn insert_payment(
        &self,
        payment: LightningPayment,
    ) -> Result<LightningPayment, DatabaseError>;
    async fn update_payment(
        &self,
        payment: LightningPayment,
    ) -> Result<LightningPayment, DatabaseError>;
}

pub trait LightningRepository:
    LightningAddressRepository + LightningPaymentRepository + LightningInvoiceRepository
{
}

// Ensure that any type that implements the individual traits also implements the new trait.
impl<T> LightningRepository for T where
    T: LightningAddressRepository + LightningPaymentRepository + LightningInvoiceRepository
{
}
