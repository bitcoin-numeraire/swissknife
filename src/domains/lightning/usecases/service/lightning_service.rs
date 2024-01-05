use crate::adapters::{database::DatabaseClient, lightning::LightningClient};

pub struct LightningService {
    pub db_client: Box<dyn DatabaseClient>,
    pub lightning_client: Box<dyn LightningClient>,
}

impl LightningService {
    pub fn new(
        db_client: Box<dyn DatabaseClient>,
        lightning_client: Box<dyn LightningClient>,
    ) -> Self {
        LightningService {
            db_client,
            lightning_client,
        }
    }
}
