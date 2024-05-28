use async_trait::async_trait;
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::lightning::entities::{
        Invoice, InvoiceFilter, LightningAddress, LightningAddressFilter, Payment, PaymentFilter,
        UserBalance,
    },
};

#[async_trait]
pub trait WalletRepository {
    async fn get_balance(
        &self,
        txn: Option<&DatabaseTransaction>,
        user: &str,
    ) -> Result<UserBalance, DatabaseError>;
}

#[async_trait]
pub trait LightningAddressRepository {
    async fn find_address(&self, id: Uuid) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_address_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_address_by_user_id(
        &self,
        user: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError>;
    async fn find_addresses(
        &self,
        filter: LightningAddressFilter,
    ) -> Result<Vec<LightningAddress>, DatabaseError>;
    async fn insert_address(
        &self,
        user: &str,
        username: &str,
    ) -> Result<LightningAddress, DatabaseError>;
    async fn delete_addresses(&self, filter: LightningAddressFilter) -> Result<u64, DatabaseError>;
}

#[async_trait]
pub trait InvoiceRepository {
    async fn find_invoice(&self, id: Uuid) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_invoice_by_payment_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<Invoice>, DatabaseError>;
    async fn find_invoices(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, DatabaseError>;
    async fn insert_invoice(
        &self,
        txn: Option<&DatabaseTransaction>,
        invoice: Invoice,
    ) -> Result<Invoice, DatabaseError>;
    async fn update_invoice(
        &self,
        txn: Option<&DatabaseTransaction>,
        invoice: Invoice,
    ) -> Result<Invoice, DatabaseError>;
    async fn delete_invoices(&self, filter: InvoiceFilter) -> Result<u64, DatabaseError>;
}

#[async_trait]
pub trait PaymentRepository {
    async fn find_payment(&self, id: Uuid) -> Result<Option<Payment>, DatabaseError>;
    async fn find_payments(&self, filter: PaymentFilter) -> Result<Vec<Payment>, DatabaseError>;
    async fn insert_payment(
        &self,
        txn: Option<&DatabaseTransaction>,
        payment: Payment,
    ) -> Result<Payment, DatabaseError>;
    async fn update_payment(&self, payment: Payment) -> Result<Payment, DatabaseError>;
    async fn delete_payments(&self, filter: PaymentFilter) -> Result<u64, DatabaseError>;
}

#[async_trait]
pub trait TransactionManager {
    async fn begin(&self) -> Result<DatabaseTransaction, DatabaseError>;
}

pub trait LightningRepository:
    LightningAddressRepository
    + PaymentRepository
    + InvoiceRepository
    + WalletRepository
    + TransactionManager
    + Sync
    + Send
{
}

// Ensure that any type that implements the individual traits also implements the new trait.
impl<T> LightningRepository for T where
    T: LightningAddressRepository
        + PaymentRepository
        + InvoiceRepository
        + WalletRepository
        + TransactionManager
        + Sync
        + Send
{
}
