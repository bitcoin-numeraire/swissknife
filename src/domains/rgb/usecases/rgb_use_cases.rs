use async_trait::async_trait;

#[async_trait]
pub trait RGBUseCases: Send + Sync {}
