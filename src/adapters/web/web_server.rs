use async_trait::async_trait;
use axum::Router;

use crate::application::errors::WebServerError;

#[async_trait]
pub trait WebServer {
    async fn start(&self) -> Result<(), WebServerError>;
    async fn nest_router(&mut self, path: &str, nested_router: Router) -> &mut Self;
}
