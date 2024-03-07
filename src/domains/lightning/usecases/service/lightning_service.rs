use crate::{
    adapters::lightning::LightningClient, domains::lightning::store::LightningAddressRepository,
};

pub struct LightningService {
    pub domain: String,
    pub store: Box<dyn LightningAddressRepository>,
    pub lightning_client: Box<dyn LightningClient>,
}

impl LightningService {
    pub fn new(
        store: Box<dyn LightningAddressRepository>,
        lightning_client: Box<dyn LightningClient>,
    ) -> Self {
        LightningService {
            store,
            lightning_client,
            domain: "numeraire.tech".to_string(),
        }
    }
}
