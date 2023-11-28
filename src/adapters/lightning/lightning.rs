use std::sync::Arc;

use async_trait::async_trait;

#[async_trait]
pub trait LightningClient {}
pub type DynLightningClient = Arc<dyn LightningClient + Send + Sync>;
