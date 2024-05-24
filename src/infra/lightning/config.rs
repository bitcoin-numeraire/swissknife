use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tracing::info;

use crate::application::errors::LightningError;

use super::{
    breez::{BreezClient, BreezClientConfig, BreezListener},
    cln::{ClnClient, ClnClientConfig},
    LightningClient,
};

#[derive(Clone, Debug, Deserialize)]
pub struct LightningConfig {
    pub domain: String,
    pub invoice_expiry: Option<u32>,
    pub invoice_description: Option<String>,
    pub provider: LightningProvider,
    pub breez_config: Option<BreezClientConfig>,
    pub cln_config: Option<ClnClientConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, EnumString, Display, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum LightningProvider {
    #[default]
    Breez,
    Cln,
}

impl LightningConfig {
    pub async fn get_client(
        &self,
        listener: Box<BreezListener>,
    ) -> Result<Box<dyn LightningClient>, LightningError> {
        match self.provider {
            LightningProvider::Breez => {
                let breez_config = self.breez_config.clone().ok_or_else(|| {
                    LightningError::MissingLightningProviderConfig(self.provider.to_string())
                })?;

                let client = BreezClient::new(breez_config.clone(), listener).await?;

                info!(
                    working_dir = %breez_config.working_dir,
                    "Lightning provider: Breez"
                );

                Ok(Box::new(client))
            }
            LightningProvider::Cln => {
                let cln_config = self.cln_config.clone().ok_or_else(|| {
                    LightningError::MissingLightningProviderConfig(self.provider.to_string())
                })?;

                let client = ClnClient::new(cln_config.clone()).await?;

                info!(
                    endpoint = %cln_config.endpoint,
                    "Lightning provider: Core Lightning"
                );

                Ok(Box::new(client))
            }
        }
    }
}
