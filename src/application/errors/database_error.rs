#[derive(Debug)]
pub enum DatabaseError {
    ParseConfig(String),
    Connect(String),
}
