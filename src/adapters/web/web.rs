use async_trait::async_trait;
use axum::routing::Router;

#[async_trait]
pub trait WebServer {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn nest_router(
        &self,
        path: &str,
        method: Router,
    ) -> Result<(), Box<dyn std::error::Error>>;
}
