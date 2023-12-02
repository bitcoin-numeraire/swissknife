use crate::application::{dtos::AppConfig, errors::ConfigError};

pub trait ConfigLoader {
    fn load(&self) -> Result<AppConfig, ConfigError>;
}
