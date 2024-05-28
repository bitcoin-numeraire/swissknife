use std::sync::Arc;

use crate::{domains::lightning::adapters::LightningRepository, infra::lightning::LightningClient};

const DEFAULT_INVOICE_EXPIRY: u32 = 3600;
const DEFAULT_INVOICE_DESCRIPTION: &str = "Numeraire Swissknife Invoice";

pub struct LightningService {
    pub domain: String,
    pub invoice_expiry: u32,
    pub invoice_description: String,
    pub store: Box<dyn LightningRepository>,
    pub lightning_client: Arc<dyn LightningClient>,
}

impl LightningService {
    pub fn new(
        store: Box<dyn LightningRepository>,
        lightning_client: Arc<dyn LightningClient>,
        domain: String,
        invoice_expiry: Option<u32>,
        invoice_description: Option<String>,
    ) -> Self {
        LightningService {
            store,
            lightning_client,
            domain,
            invoice_expiry: invoice_expiry.unwrap_or(DEFAULT_INVOICE_EXPIRY),
            invoice_description: invoice_description.unwrap_or(DEFAULT_INVOICE_DESCRIPTION.into()),
        }
    }
}
