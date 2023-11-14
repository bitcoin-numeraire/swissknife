use async_trait::async_trait;

#[async_trait]
pub trait WebServer {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
}
