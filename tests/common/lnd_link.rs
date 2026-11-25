//! A private loopback link to regtest LND. Stopping it drops only the selected
//! SwissKnife server's connections, while other integration tests keep running.

use std::net::SocketAddr;
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};

pub struct LndLink {
    addr: SocketAddr,
    task: JoinHandle<()>,
}

impl LndLink {
    pub async fn connect() -> Self {
        let port = std::env::var("SWISSKNIFE_ITEST_LND_GRPC_PORT")
            .unwrap_or_else(|_| "10009".into())
            .parse::<u16>()
            .expect("regtest LND port");
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind regtest link");
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (mut client, _) = accepted.expect("accept link");
                        connections.spawn(async move {
                            let mut node = TcpStream::connect(("127.0.0.1", port)).await.expect("connect to regtest LND");
                            let _ = copy_bidirectional(&mut client, &mut node).await;
                        });
                    }
                    Some(result) = connections.join_next(), if !connections.is_empty() => {
                        result.expect("forward regtest connection");
                    }
                }
            }
        });
        Self { addr, task }
    }

    pub fn endpoint(&self) -> String {
        format!("https://{}", self.addr)
    }

    pub async fn disconnect(mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}

impl Drop for LndLink {
    fn drop(&mut self) {
        self.task.abort();
    }
}
