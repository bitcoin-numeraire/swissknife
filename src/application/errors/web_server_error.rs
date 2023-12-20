#[derive(Debug)]
pub enum WebServerError {
    Listener(String),
    Serve(String),
}
