use std::sync::Arc;

use super::{LightningAddressRepository, LightningInvoiceRepository, LightningPaymentRepository};

#[derive(Clone)]
pub struct LightningStore {
    pub invoice_repo: Arc<dyn LightningInvoiceRepository>,
    pub address_repo: Arc<dyn LightningAddressRepository>,
    pub payment_repo: Arc<dyn LightningPaymentRepository>,
}

impl LightningStore {
    pub fn new(
        invoice_repo: Arc<dyn LightningInvoiceRepository>,
        address_repo: Arc<dyn LightningAddressRepository>,
        payment_repo: Arc<dyn LightningPaymentRepository>,
    ) -> Self {
        Self {
            address_repo,
            invoice_repo,
            payment_repo,
        }
    }
}
