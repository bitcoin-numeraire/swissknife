use crate::{
    adapters::lightning::LightningClient,
    domains::lightning::store::{LightningAddressRepository, LightningInvoiceRepository},
};

pub struct LightningService {
    pub domain: String,
    pub invoice_repo: Box<dyn LightningInvoiceRepository>,
    pub address_repo: Box<dyn LightningAddressRepository>,
    pub lightning_client: Box<dyn LightningClient>,
}

impl LightningService {
    pub fn new(
        invoice_repo: Box<dyn LightningInvoiceRepository>,
        address_repo: Box<dyn LightningAddressRepository>,
        lightning_client: Box<dyn LightningClient>,
        domain: String,
    ) -> Self {
        LightningService {
            invoice_repo,
            address_repo,
            lightning_client,
            domain,
        }
    }
}
