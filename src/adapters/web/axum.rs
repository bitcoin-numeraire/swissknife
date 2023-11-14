use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use axum::{routing::get, Router, Server};
use tokio::sync::Mutex;

use super::web::WebServer;
pub struct AxumServerConfig {
    pub addr: String,
}

pub struct AxumServer {
    addr: SocketAddr,
    router: Arc<Mutex<Router>>,
}

impl AxumServer {
    pub fn new(config: AxumServerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let router = Arc::new(Mutex::new(Router::new().route("/", get(Self::root))));
        let addr: SocketAddr = config.addr.parse()?;

        Ok(Self { router, addr })
    }

    async fn root() -> &'static str {
        "Hello, World!"
    }
}

#[async_trait]
impl WebServer for AxumServer {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let router = {
            let lock = self.router.lock().await;
            lock.clone()
        };

        Server::bind(&self.addr)
            .serve(router.into_make_service())
            .await?;

        Ok(())
    }
}
