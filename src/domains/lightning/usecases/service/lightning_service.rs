use crate::{adapters::lightning::LightningClient, domains::lightning::store::LightningStore};

pub struct LightningService {
    pub domain: String,
    pub store: LightningStore,
    pub lightning_client: Box<dyn LightningClient>,
}

impl LightningService {
    pub fn new(store: LightningStore, lightning_client: Box<dyn LightningClient>) -> Self {
        LightningService {
            store,
            lightning_client,
            domain: "numeraire.tech".to_string(),
        }
    }
}
