use crate::adapters::{auth::DynAuthenticator, lightning::DynLightningClient, rgb::DynRGBClient};

#[derive(Clone)]
pub struct AppState {
    pub auth_enabled: bool,
    pub jwt_validator: DynAuthenticator,
    pub lightning_client: DynLightningClient,
    pub rgb_client: DynRGBClient,
}
