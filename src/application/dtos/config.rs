use serde::Deserialize;

use crate::adapters::{
    lightning::breez::BreezClientConfig, rgb::rgblib::RGBLibClientConfig,
    web::axum::AxumServerConfig,
};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub lightning: BreezClientConfig,
    pub rgb: RGBLibClientConfig,
    pub web: AxumServerConfig,
}
