use crate::adapters::{
    auth::DynAuthenticator, database::DynDatabaseClient, lightning::DynLightningClient,
    rgb::DynRGBClient,
};

#[derive(Clone)]
pub struct AppState {
    pub db_client: DynDatabaseClient,
    pub auth_enabled: bool,
    pub jwt_validator: DynAuthenticator,
    pub lightning_client: DynLightningClient,
    pub rgb_client: DynRGBClient,
}
