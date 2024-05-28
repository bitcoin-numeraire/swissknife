use std::sync::Arc;

use crate::{domains::lightning::adapters::LightningRepository, infra::lightning::LightningClient};

const DEFAULT_INVOICE_EXPIRY: u32 = 3600;

pub struct LightningService {
    pub domain: String,
    pub invoice_expiry: u32,
    pub store: Box<dyn LightningRepository>,
    pub lightning_client: Arc<dyn LightningClient>,
}

impl LightningService {
    pub fn new(
        store: Box<dyn LightningRepository>,
        lightning_client: Arc<dyn LightningClient>,
        domain: String,
        invoice_expiry: Option<u32>,
    ) -> Self {
        LightningService {
            store,
            lightning_client,
            domain,
            invoice_expiry: invoice_expiry.unwrap_or(DEFAULT_INVOICE_EXPIRY),
        }
    }
}
