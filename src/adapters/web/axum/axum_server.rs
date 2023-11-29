use std::{
    net::{AddrParseError, SocketAddr},
    pin::Pin,
    sync::Arc,
};

use async_trait::async_trait;
use axum::{Router, Server};
use futures_util::future::poll_fn;
use hyper::{
    server::{
        accept::Accept,
        conn::{AddrIncoming, Http},
    },
    service::Service,
};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};

use crate::{
    adapters::web::WebServer,
    application::errors::{ApplicationError, AsyncError, ConfigError, WebServerError},
};

use super::AxumServerConfig;

pub struct AxumServer {
    addr: SocketAddr,
    router: Arc<Mutex<Option<Router>>>,
    tls_config: Option<Arc<ServerConfig>>,
}

impl AxumServer {
    pub fn new(config: AxumServerConfig) -> Result<Self, ConfigError> {
        let router = Arc::new(Mutex::new(Some(Router::new())));
        let addr: SocketAddr = config
            .addr
            .parse()
            .map_err(|e: AddrParseError| ConfigError::WebServer(e.to_string()))?;

        Ok(Self {
            router,
            addr,
            tls_config: config.tls_config(),
        })
    }
}

#[async_trait]
impl WebServer for AxumServer {
    async fn start(&self) -> Result<(), ApplicationError> {
        let router = {
            let lock = self.router.lock().await;
            lock.clone()
                .ok_or("router is missing")
                .map_err(|e| AsyncError::Mutex(e.to_string()))?
        };

        if let Some(tls_config) = self.tls_config.clone() {
            let acceptor = TlsAcceptor::from(tls_config);

            let listener = TcpListener::bind(self.addr).await.unwrap();
            let mut listener = AddrIncoming::from_listener(listener).unwrap();

            let protocol = Arc::new(Http::new());

            let mut app = router.into_make_service_with_connect_info::<SocketAddr>();

            loop {
                let stream = poll_fn(|cx| Pin::new(&mut listener).poll_accept(cx))
                    .await
                    .unwrap()
                    .unwrap();

                let acceptor = acceptor.clone();
                let protocol = protocol.clone();

                let svc = app.call(&stream);

                tokio::spawn(async move {
                    if let Ok(stream) = acceptor.accept(stream).await {
                        let _ = protocol.serve_connection(stream, svc.await.unwrap()).await;
                    }
                });
            }
        } else {
            Server::bind(&self.addr)
                .serve(router.into_make_service())
                .await
                .map_err(|e| WebServerError::Listener(e.to_string()))?;

            Ok(())
        }
    }

    async fn nest_router(&self, path: &str, method: Router) -> Result<(), ApplicationError> {
        let mut lock = self.router.lock().await;
        let router = lock
            .take()
            .ok_or("router is missing")
            .map_err(|e| AsyncError::Mutex(e.to_string()))?;

        *lock = Some(router.nest(path, method));

        Ok(())
    }
}
