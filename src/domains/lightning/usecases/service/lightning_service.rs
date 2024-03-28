use crate::{
    adapters::lightning::LightningClient,
    domains::lightning::store::{
        LightningAddressRepository, LightningInvoiceRepository, LightningPaymentRepository,
    },
};

pub struct LightningService {
    pub domain: String,
    pub invoice_repo: Box<dyn LightningInvoiceRepository>,
    pub address_repo: Box<dyn LightningAddressRepository>,
    pub payment_repo: Box<dyn LightningPaymentRepository>,
    pub lightning_client: Box<dyn LightningClient>,
}

impl LightningService {
    pub fn new(
        invoice_repo: Box<dyn LightningInvoiceRepository>,
        address_repo: Box<dyn LightningAddressRepository>,
        payment_repo: Box<dyn LightningPaymentRepository>,
        lightning_client: Box<dyn LightningClient>,
        domain: String,
    ) -> Self {
        LightningService {
            invoice_repo,
            address_repo,
            payment_repo,
            lightning_client,
            domain,
        }
    }
}
