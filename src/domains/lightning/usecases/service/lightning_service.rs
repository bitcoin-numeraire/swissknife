use crate::{domains::lightning::adapters::LightningRepository, infra::lightning::LightningClient};

pub struct LightningService {
    pub domain: String,
    pub store: Box<dyn LightningRepository>,
    pub lightning_client: Box<dyn LightningClient>,
}

impl LightningService {
    pub fn new(
        store: Box<dyn LightningRepository>,
        lightning_client: Box<dyn LightningClient>,
        domain: String,
    ) -> Self {
        LightningService {
            store,
            lightning_client,
            domain,
        }
    }
}
