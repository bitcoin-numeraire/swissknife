use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use axum::{Router, Server};
use tokio::sync::Mutex;

use super::web::WebServer;
pub struct AxumServerConfig {
    pub addr: String,
}

pub struct AxumServer {
    addr: SocketAddr,
    router: Arc<Mutex<Option<Router>>>,
}

impl AxumServer {
    pub fn new(config: AxumServerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let router = Arc::new(Mutex::new(Some(Router::new())));
        let addr: SocketAddr = config.addr.parse()?;

        Ok(Self { router, addr })
    }
}

#[async_trait]
impl WebServer for AxumServer {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let router = {
            let lock = self.router.lock().await;
            lock.clone().ok_or("router is missing")?
        };

        Server::bind(&self.addr)
            .serve(router.into_make_service())
            .await?;

        Ok(())
    }

    async fn nest_router(
        &self,
        path: &str,
        method: Router,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lock = self.router.lock().await;
        let router = lock.take().ok_or("router is missing")?;

        *lock = Some(router.nest(path, method));

        Ok(())
    }
}
