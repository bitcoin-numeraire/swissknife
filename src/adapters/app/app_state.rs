use std::sync::Arc;

use crate::adapters::{
    auth::Authenticator, database::DatabaseClient, lightning::LightningClient, rgb::RGBClient,
};

#[derive(Clone)]
pub struct AppState {
    pub db_client: Arc<dyn DatabaseClient>,
    pub jwt_authenticator: Option<Arc<dyn Authenticator>>,
    pub lightning_client: Arc<dyn LightningClient>,
    pub rgb_client: Arc<dyn RGBClient>,
}
