use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebServerError {
    Listener(String),
    Serve(String),
}
