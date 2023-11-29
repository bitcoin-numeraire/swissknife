use async_trait::async_trait;
use axum::routing::Router;

use crate::application::errors::ApplicationError;

#[async_trait]
pub trait WebServer {
    async fn start(&self) -> Result<(), ApplicationError>;
    async fn nest_router(&self, path: &str, method: Router) -> Result<(), ApplicationError>;
}
