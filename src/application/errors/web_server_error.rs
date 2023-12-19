#[derive(Debug)]
pub enum WebServerError {
    Parse(String),
    Listener(String),
}
